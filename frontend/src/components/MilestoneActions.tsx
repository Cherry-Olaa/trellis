import { useRef, useState } from 'react'
import { nativeToScVal, xdr } from '@stellar/stellar-sdk'
import { useContractInvoke } from '../hooks/useContractInvoke'
import { useWallet } from '../context/WalletContext'
import { useToastActions } from '../hooks/useToast'
import TransactionStatus from './TransactionStatus'
import { hexToBytes } from '../lib/format'
import type { Agreement, Milestone } from '../lib/soroban'

interface MilestoneActionsProps {
  milestone: Milestone
  agreement: Agreement
  onSuccess?: () => void
}

/**
 * Encodes `proof_uri` for the contract's `Option<String>` parameter.
 * A blank input means "no proof link" and maps to None (ScVal void), never
 * to an empty string.
 */
export function proofUriToScVal(proofUri: string): xdr.ScVal {
  const trimmed = proofUri.trim()
  return trimmed ? nativeToScVal(trimmed, { type: 'string' }) : xdr.ScVal.scvVoid()
}

const MAX_RETRIES = 3
const BASE_FEE = 100_000

export default function MilestoneActions({ milestone, agreement, onSuccess }: MilestoneActionsProps) {
  const wallet = useWallet()
  const toast = useToastActions()
  const { invoke, status, txHash, error, reset } = useContractInvoke()
  const [showProofInput, setShowProofInput] = useState(false)
  const [proofUri, setProofUri] = useState('')
  const [showConfirm, setShowConfirm] = useState<string | null>(null)
  const [isRetrying, setIsRetrying] = useState(false)
  const pendingAction = useRef<string | null>(null)

  const isLoading = status === 'building' || status === 'signing' || status === 'submitting'
  const isUserPayer = wallet.publicKey === agreement.payer
  const isUserPayee = wallet.publicKey === agreement.payee

  // lock_funds, submit_work and approve_and_release take no caller argument —
  // the contract derives the signer from agreement.payer / agreement.payee via
  // require_auth(). Only raise_dispute takes an explicit leading `caller`.
  const handleLockFunds = async (fee?: string) => {
    if (!wallet.publicKey) return

    pendingAction.current = 'lock_funds'
    try {
      const idBytes = hexToBytes(agreement.agreement_id)
      const args = [
        nativeToScVal(idBytes, { type: 'bytes' }),
        nativeToScVal(milestone.id, { type: 'u32' }),
      ]

      await invoke('lock_funds', args, wallet.publicKey, fee)
      toast.success({ title: 'Funds locked successfully' })
      setShowConfirm(null)
      setRetryCount(0)
      pendingAction.current = null
      onSuccess?.()
    } catch (err) {
      toast.error({ title: 'Lock funds failed', message: err instanceof Error ? err.message : 'Unknown error' })
    }
  }

  const handleSubmitWork = async (fee?: string) => {
    if (!wallet.publicKey) return

    pendingAction.current = 'submit_work'
    try {
      const idBytes = hexToBytes(agreement.agreement_id)
      const args = [
        nativeToScVal(idBytes, { type: 'bytes' }),
        nativeToScVal(milestone.id, { type: 'u32' }),
        proofUriToScVal(proofUri),
      ]

      await invoke('submit_work', args, wallet.publicKey, fee)
      toast.success({ title: 'Work submitted successfully' })
      setShowProofInput(false)
      setProofUri('')
      setShowConfirm(null)
      setRetryCount(0)
      pendingAction.current = null
      onSuccess?.()
    } catch (err) {
      toast.error({ title: 'Submit work failed', message: err instanceof Error ? err.message : 'Unknown error' })
    }
  }

  const handleApproveRelease = async (fee?: string) => {
    if (!wallet.publicKey) return

    pendingAction.current = 'approve_and_release'
    try {
      const idBytes = hexToBytes(agreement.agreement_id)
      const args = [
        nativeToScVal(idBytes, { type: 'bytes' }),
        nativeToScVal(milestone.id, { type: 'u32' }),
      ]

      await invoke('approve_and_release', args, wallet.publicKey, fee)
      toast.success({ title: 'Milestone approved and funds released' })
      setShowConfirm(null)
      setRetryCount(0)
      pendingAction.current = null
      onSuccess?.()
    } catch (err) {
      toast.error({ title: 'Approve failed', message: err instanceof Error ? err.message : 'Unknown error' })
    }
  }

  const handleRaiseDispute = async (fee?: string) => {
    if (!wallet.publicKey) return

    pendingAction.current = 'raise_dispute'
    try {
      const idBytes = hexToBytes(agreement.agreement_id)
      const args = [
        nativeToScVal(wallet.publicKey, { type: 'address' }),
        nativeToScVal(idBytes, { type: 'bytes' }),
        nativeToScVal(milestone.id, { type: 'u32' }),
      ]

      await invoke('raise_dispute', args, wallet.publicKey, fee)
      toast.success({ title: 'Dispute raised' })
      setShowConfirm(null)
      setRetryCount(0)
      pendingAction.current = null
      onSuccess?.()
    } catch (err) {
      toast.error({ title: 'Raise dispute failed', message: err instanceof Error ? err.message : 'Unknown error' })
    }
  }

  const handleRetry = async () => {
    if (isRetrying) return
    setIsRetrying(true)
    try {
      reset()
      if (pendingAction.current === 'lock_funds') await handleLockFunds()
      else if (pendingAction.current === 'submit_work') await handleSubmitWork()
      else if (pendingAction.current === 'approve_and_release') await handleApproveRelease()
      else if (pendingAction.current === 'raise_dispute') await handleRaiseDispute()
    } catch (err) {
      toast.error({ title: 'Retry failed', message: err instanceof Error ? err.message : 'Unknown error' })
    } finally {
      setIsRetrying(false)
    }
  }

  // Determine available actions based on milestone status and user role
  const actions: Array<{ label: string; action: () => void; color: string }> = []

  if (milestone.status === 'Pending' && isUserPayer) {
    actions.push({
      label: 'Lock Funds',
      action: () => setShowConfirm('lock'),
      color: 'bg-cyan-500 hover:bg-cyan-400',
    })
  }

  if (milestone.status === 'Funded' && isUserPayee) {
    actions.push({
      label: 'Submit Work',
      action: () => setShowProofInput(true),
      color: 'bg-emerald-500 hover:bg-emerald-400',
    })
  }

  if (milestone.status === 'WorkSubmitted' && isUserPayer) {
    actions.push({
      label: 'Approve & Release',
      action: () => setShowConfirm('approve'),
      color: 'bg-green-500 hover:bg-green-400',
    })
  }

  const isDisputable = milestone.status === 'Funded' || milestone.status === 'WorkSubmitted'
  // raise_dispute rejects any caller other than the payer or payee.
  const isUserParty = isUserPayer || isUserPayee

  if (isDisputable && wallet.connected && isUserParty) {
    actions.push({
      label: 'Dispute',
      action: () => setShowConfirm('dispute'),
      color: 'bg-red-500 hover:bg-red-400',
    })
  }

  if (actions.length === 0) {
    if (isDisputable && wallet.connected && !isUserParty) {
      return (
        <span className="text-gray-500 dark:text-gray-500 light:text-gray-600 text-xs">
          Only the payer or payee can act on this milestone.
        </span>
      )
    }
    return <span className="text-gray-500 dark:text-gray-500 light:text-gray-600 text-sm">—</span>
  }

  return (
    <div className="space-y-2">
      <div className="flex gap-2 flex-wrap">
        {actions.map((action) => (
          <button
            key={action.label}
            onClick={action.action}
            disabled={isLoading || !wallet.connected}
            className={`px-3 py-1 ${action.color} text-white dark:text-white light:text-gray-900 font-semibold text-xs rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed`}
          >
            {action.label}
          </button>
        ))}
      </div>

      <TransactionStatus
        status={status}
        txHash={txHash}
        error={error}
        onRetry={handleRetry}
        method={pendingAction.current || ''}
        retrying={isRetrying}
      />

      {/* Proof URI Input */}
      {showProofInput && (
        <div className="space-y-2 p-3 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 rounded">
          {/* maxLength mirrors the contract's MAX_PROOF_URI_LEN (512 bytes) —
              longer proofs are rejected on-chain with ProofUriTooLong. */}
          <input
            type="text"
            placeholder="Proof URI (optional, e.g., GitHub PR, IPFS link)"
            maxLength={512}
            value={proofUri}
            onChange={(e) => setProofUri(e.target.value)}
            className="w-full px-3 py-2 bg-navy-800 dark:bg-navy-800 light:bg-white border border-navy-600 dark:border-navy-600 light:border-gray-300 text-white dark:text-white light:text-gray-900 text-sm rounded focus:outline-none focus:border-cyan-400"
          />
          <div className="flex gap-2">
            <button
              onClick={handleSubmitWork}
              disabled={isLoading}
              className="px-3 py-1 bg-emerald-500 hover:bg-emerald-400 text-white dark:text-white light:text-gray-900 text-xs rounded disabled:opacity-50"
            >
              {isLoading ? 'Submitting...' : 'Submit'}
            </button>
            <button
              onClick={() => {
                setShowProofInput(false)
                setProofUri('')
              }}
              className="px-3 py-1 bg-navy-600 dark:bg-navy-600 light:bg-gray-200 hover:bg-navy-500 dark:hover:bg-navy-500 light:hover:bg-gray-300 text-white dark:text-white light:text-gray-900 text-xs rounded"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Confirmation Modal */}
      {showConfirm && (
        <div className="p-3 bg-navy-700 dark:bg-navy-700 light:bg-gray-100 rounded border border-yellow-500/50">
          <p className="text-sm text-gray-300 dark:text-gray-300 light:text-gray-700 mb-2">
            {showConfirm === 'lock' && `Lock ${milestone.amount} tokens for milestone ${milestone.id}?`}
            {showConfirm === 'approve' && `Approve and release ${milestone.amount} tokens to payee?`}
            {showConfirm === 'dispute' && 'Raise a dispute for this milestone?'}
          </p>
          <div className="flex gap-2">
            <button
              onClick={() => {
                if (showConfirm === 'lock') handleLockFunds()
                else if (showConfirm === 'approve') handleApproveRelease()
                else if (showConfirm === 'dispute') handleRaiseDispute()
              }}
              disabled={isLoading}
              className="px-3 py-1 bg-cyan-500 hover:bg-cyan-400 text-white dark:text-white light:text-gray-900 text-xs rounded disabled:opacity-50"
            >
              {isLoading ? 'Processing...' : 'Confirm'}
            </button>
            <button
              onClick={() => setShowConfirm(null)}
              disabled={isLoading}
              className="px-3 py-1 bg-navy-600 dark:bg-navy-600 light:bg-gray-200 hover:bg-navy-500 dark:hover:bg-navy-500 light:hover:bg-gray-300 text-white dark:text-white light:text-gray-900 text-xs rounded disabled:opacity-50"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
