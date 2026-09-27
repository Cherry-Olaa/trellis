import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { nativeToScVal, StrKey, xdr } from '@stellar/stellar-sdk'
import { useContractInvoke } from '../hooks/useContractInvoke'
import { useWallet } from '../context/WalletContext'
import { useToastActions } from '../hooks/useToast'
import MilestoneBuilder, { type MilestoneInput } from '../components/MilestoneBuilder'
import { AgreementIdGenerator } from '../components/AgreementIdGenerator'
import { agreementIdToScVal, milestoneToScVal } from '../lib/soroban'
import { generateAgreementId } from '../lib/agreementId'

export interface FormData {
  payer: string
  payee: string
  resolver: string
  token: string
}

interface ValidationErrors {
  payer?: string
  payee?: string
  resolver?: string
  token?: string
  milestones?: string
}

/**
 * Converts a user-supplied decimal amount string to a BigInt stroops value
 * (7 decimal places) without floating-point arithmetic.
 *
 * Examples:
 *   "100"         → 1000000000n
 *   "100.5"       → 1005000000n
 *   "0.0000001"   → 1n
 *   "100.12345678" → throws (more than 7 decimal places)
 *
 * @throws {Error} if the amount is not a valid positive decimal number or has
 *   more than 7 decimal places.
 */
export function amountToStroops(amount: string): bigint {
  const trimmed = amount.trim()
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    throw new Error(`Invalid amount: "${trimmed}" is not a valid positive number.`)
  }

  const [intPart, fracPart = ''] = trimmed.split('.')
  const DECIMALS = 7

  if (fracPart.length > DECIMALS) {
    throw new Error(
      `Invalid amount: "${trimmed}" has more than ${DECIMALS} decimal places.`
    )
  }

  // Pad fractional part to exactly 7 digits.
  const paddedFrac = fracPart.padEnd(DECIMALS, '0')
  return BigInt(intPart + paddedFrac)
}

/**
 * Builds the argument list for the contract's `init` entrypoint, in its exact
 * signature order: (agreement_id, payer, payee, token, milestones, dispute_resolver).
 *
 * Each milestone is encoded as the on-chain `Milestone` struct — `amount` in
 * stroops, `status: Pending`, `proof_uri: None`. The description is local-only
 * metadata with no on-chain field, so it is not sent.
 */
export function buildInitArgs(
  agreementId: string,
  formData: FormData,
  milestones: MilestoneInput[],
): xdr.ScVal[] {
  const milestoneScVals = milestones.map((m) =>
    milestoneToScVal({
      amount: amountToStroops(m.amount), // Convert to stroops (7 decimals) without float imprecision
      status: 'Pending',
      proof_uri: null,
    })
  )

  return [
    agreementIdToScVal(agreementId),
    nativeToScVal(formData.payer.trim(), { type: 'address' }),
    nativeToScVal(formData.payee.trim(), { type: 'address' }),
    nativeToScVal(formData.token.trim(), { type: 'address' }),
    xdr.ScVal.scvVec(milestoneScVals),
    nativeToScVal(formData.resolver.trim(), { type: 'address' }),
  ]
}

function CreateAgreementPage() {
  const navigate = useNavigate()
  const wallet = useWallet()
  const toast = useToastActions()
  const { invoke, status } = useContractInvoke()

  const [agreementId, setAgreementId] = useState<string>('')
  const [formData, setFormData] = useState<FormData>({
    payer: '',
    payee: '',
    resolver: '',
    token: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC', // Default testnet USDC
  })

  const [milestones, setMilestones] = useState<MilestoneInput[]>([])
  const [errors, setErrors] = useState<ValidationErrors>({})
  const [isSubmitting, setIsSubmitting] = useState(false)

  const validateAddress = (address: string, field: string): string | undefined => {
    const trimmed = address.trim()
    if (!trimmed) return `${field} is required`

    if (trimmed.startsWith('C')) {
      return StrKey.isValidContractKey(trimmed)
        ? undefined
        : `${field} must be a valid contract address (starts with C)`
    }

    try {
      StrKey.decodeEd25519PublicKey(trimmed)
      return undefined
    } catch {
      return `${field} must be a valid account address (starts with G)`
    }
  }

  const validateForm = (): boolean => {
    const newErrors: ValidationErrors = {}

    newErrors.payer = validateAddress(formData.payer, 'Payer')
    newErrors.payee = validateAddress(formData.payee, 'Payee')
    newErrors.resolver = validateAddress(formData.resolver, 'Resolver')
    newErrors.token = validateAddress(formData.token, 'Token')

    if (milestones.length === 0) {
      newErrors.milestones = 'At least one milestone is required'
    } else {
      const invalidMilestone = milestones.find((m) => {
        if (!m.amount) return true
        const trimmed = m.amount.trim()
        // Must be a valid positive decimal with at most 7 decimal places.
        if (!/^\d+(\.\d{1,7})?$/.test(trimmed)) return true
        // Must be greater than zero.
        try {
          return amountToStroops(trimmed) <= 0n
        } catch {
          return true
        }
      })
      if (invalidMilestone) {
        newErrors.milestones =
          'All milestones must have valid positive amounts with at most 7 decimal places'
      }
    }

    setErrors(newErrors)
    return Object.values(newErrors).every((err) => !err)
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    if (!wallet.connected || !wallet.publicKey) {
      toast.error({ title: 'Wallet not connected', message: 'Please connect your wallet first' })
      return
    }

    if (!validateForm()) {
      toast.error({ title: 'Validation failed', message: 'Please fix the errors and try again' })
      return
    }

    setIsSubmitting(true)

    try {
      // Fall back to a fresh ID if the user never clicked "Generate".
      const id = agreementId || generateAgreementId()
      const txHash = await invoke('init', buildInitArgs(id, formData, milestones), wallet.publicKey)

      toast.success({ title: 'Agreement created!', message: `Transaction: ${txHash.slice(0, 8)}...` })

      setTimeout(() => {
        navigate(`/agreement/${id}`)
      }, 1500)
    } catch (err) {
      console.error('Failed to create agreement:', err)
      toast.error({
        title: 'Creation failed',
        message: err instanceof Error ? err.message : 'Unknown error occurred',
      })
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleInputChange = (field: keyof FormData, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }))
    
    // Clear error for this field
    if (errors[field]) {
      setErrors((prev) => ({ ...prev, [field]: undefined }))
    }
  }

  return (
    <main className="px-6 pt-16 pb-32 max-w-3xl mx-auto">
      <h1 className="text-3xl font-bold text-white dark:text-white light:text-gray-900">Create Agreement</h1>
      <p className="mt-3 text-gray-400 dark:text-gray-400 light:text-gray-600">
        Define the payer, payee, resolver and milestones for a new escrow agreement.
      </p>

      {/* Agreement ID Generator — generate a shareable ID for counterparties */}
      <div className="mt-10">
        <AgreementIdGenerator onGenerate={setAgreementId} />
      </div>

      {agreementId && (
        <div className="mt-4 px-4 py-3 bg-navy-700/50 dark:bg-navy-700/50 light:bg-gray-100 border border-navy-600 dark:border-navy-600 light:border-gray-300 rounded-lg text-sm text-gray-400 dark:text-gray-400 light:text-gray-600 flex items-center gap-2">
          <svg
            className="w-4 h-4 text-cyan-400 dark:text-cyan-400 light:text-cyan-600 shrink-0"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          <span>
            Agreement ID generated. Share it with your counterparty so they can track the agreement.
          </span>
        </div>
      )}

      <form onSubmit={handleSubmit} className="mt-10 space-y-6">
        {/* Party Addresses */}
        <div className="rounded-xl border border-navy-700 dark:border-navy-700 light:border-gray-200 bg-navy-800/60 dark:bg-navy-800/60 light:bg-gray-50 p-6 space-y-4">
          <h2 className="text-lg font-semibold text-white dark:text-white light:text-gray-900">Agreement Parties</h2>

          <div>
            <label className="block text-sm text-gray-400 dark:text-gray-400 light:text-gray-600 mb-1">
              Payer Address <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={formData.payer}
              onChange={(e) => handleInputChange('payer', e.target.value)}
              placeholder="G..."
              className={`w-full px-4 py-2 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 border ${
                errors.payer ? 'border-red-500' : 'border-navy-600'
              } text-white dark:text-white light:text-gray-900 rounded-lg focus:outline-none focus:border-cyan-400`}
            />
            {errors.payer && <p className="mt-1 text-xs text-red-400">{errors.payer}</p>}
          </div>

          <div>
            <label className="block text-sm text-gray-400 dark:text-gray-400 light:text-gray-600 mb-1">
              Payee Address <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={formData.payee}
              onChange={(e) => handleInputChange('payee', e.target.value)}
              placeholder="G..."
              className={`w-full px-4 py-2 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 border ${
                errors.payee ? 'border-red-500' : 'border-navy-600'
              } text-white dark:text-white light:text-gray-900 rounded-lg focus:outline-none focus:border-cyan-400`}
            />
            {errors.payee && <p className="mt-1 text-xs text-red-400">{errors.payee}</p>}
          </div>

          <div>
            <label className="block text-sm text-gray-400 dark:text-gray-400 light:text-gray-600 mb-1">
              Dispute Resolver Address <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={formData.resolver}
              onChange={(e) => handleInputChange('resolver', e.target.value)}
              placeholder="G..."
              className={`w-full px-4 py-2 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 border ${
                errors.resolver ? 'border-red-500' : 'border-navy-600'
              } text-white dark:text-white light:text-gray-900 rounded-lg focus:outline-none focus:border-cyan-400`}
            />
            {errors.resolver && <p className="mt-1 text-xs text-red-400">{errors.resolver}</p>}
          </div>

          <div>
            <label className="block text-sm text-gray-400 dark:text-gray-400 light:text-gray-600 mb-1">
              Token Contract Address <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={formData.token}
              onChange={(e) => handleInputChange('token', e.target.value)}
              placeholder="C..."
              className={`w-full px-4 py-2 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 border ${
                errors.token ? 'border-red-500' : 'border-navy-600'
              } text-white dark:text-white light:text-gray-900 rounded-lg focus:outline-none focus:border-cyan-400`}
            />
            {errors.token && <p className="mt-1 text-xs text-red-400">{errors.token}</p>}
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-500 light:text-gray-600">Default: Testnet USDC</p>
          </div>
        </div>

        {/* Milestones */}
        <div className="rounded-xl border border-navy-700 dark:border-navy-700 light:border-gray-200 bg-navy-800/60 dark:bg-navy-800/60 light:bg-gray-50 p-6">
          <MilestoneBuilder milestones={milestones} onChange={setMilestones} />
          {errors.milestones && (
            <p className="mt-2 text-sm text-red-400">{errors.milestones}</p>
          )}
        </div>

        {/* Submit Button */}
        <div className="flex gap-4">
          <button
            type="submit"
            disabled={isSubmitting || !wallet.connected || status !== 'idle'}
            className="flex-1 px-6 py-3 bg-cyan-500 hover:bg-cyan-400 text-navy-900 font-semibold rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? 'Creating Agreement...' : 'Create Agreement'}
          </button>
          
          <button
            type="button"
            onClick={() => navigate('/')}
            disabled={isSubmitting}
            className="px-6 py-3 border border-navy-700 dark:border-navy-700 light:border-gray-200 text-gray-400 dark:text-gray-400 light:text-gray-600 hover:text-white dark:hover:text-white light:hover:text-gray-900 rounded-lg transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
        </div>

        {!wallet.connected && (
          <p className="text-sm text-yellow-400 text-center">
            Please connect your wallet to create an agreement
          </p>
        )}
      </form>
    </main>
  )
}

export default CreateAgreementPage
