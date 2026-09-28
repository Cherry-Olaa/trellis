import { ExplorerLink } from './ExplorerLink'
import MilestoneActions from './MilestoneActions'
import CopyButton from './CopyButton'
import type { Agreement } from '../lib/soroban'

interface AgreementDetailProps {
  agreement: Agreement
  onUpdate?: () => void
}

export default function AgreementDetail({ agreement, onUpdate }: AgreementDetailProps) {
  return (
    <div className="space-y-6">
      {/* Agreement Header */}
      <div className="rounded-xl border border-navy-700 dark:border-navy-700 light:border-gray-200 bg-navy-800/60 dark:bg-navy-800/60 light:bg-gray-50 p-6">
        <h2 className="text-xl font-semibold text-white dark:text-white light:text-gray-900 mb-4">Agreement Details</h2>

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-gray-400 dark:text-gray-400 light:text-gray-600">Agreement ID</span>
            <div className="flex items-center gap-2">
              <ExplorerLink type="contract" value={agreement.agreement_id} />
              <CopyButton text={agreement.agreement_id} label="Copy agreement ID" />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-gray-400 dark:text-gray-400 light:text-gray-600">Payer</span>
            <div className="flex items-center gap-2">
              <ExplorerLink type="account" value={agreement.payer} />
              <CopyButton text={agreement.payer} label="Copy payer address" />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-gray-400 dark:text-gray-400 light:text-gray-600">Payee</span>
            <div className="flex items-center gap-2">
              <ExplorerLink type="account" value={agreement.payee} />
              <CopyButton text={agreement.payee} label="Copy payee address" />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-gray-400 dark:text-gray-400 light:text-gray-600">Resolver</span>
            <div className="flex items-center gap-2">
              <ExplorerLink type="account" value={agreement.dispute_resolver} />
              <CopyButton text={agreement.dispute_resolver} label="Copy resolver address" />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-gray-400 dark:text-gray-400 light:text-gray-600">Token</span>
            <ExplorerLink type="contract" value={agreement.token} />
          </div>
        </div>
      </div>

      {/* Milestones Table */}
      <div className="rounded-xl border border-navy-700 dark:border-navy-700 light:border-gray-200 bg-navy-800/60 dark:bg-navy-800/60 light:bg-gray-50 overflow-hidden">
        <div className="p-6 border-b border-navy-700 dark:border-navy-700 light:border-gray-200">
          <h2 className="text-xl font-semibold text-white dark:text-white light:text-gray-900">Milestones</h2>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-navy-700/50 dark:bg-navy-700/50 light:bg-gray-100 text-gray-400 dark:text-gray-400 light:text-gray-600 text-xs uppercase">
              <tr>
                <th className="py-3 px-4 text-left">#</th>
                <th className="py-3 px-4 text-left">Amount</th>
                <th className="py-3 px-4 text-left">Status</th>
                <th className="py-3 px-4 text-left">Proof</th>
                <th className="py-3 px-4 text-left">Actions</th>
              </tr>
            </thead>
            <tbody>
              {agreement.milestones.map((milestone) => (
                <tr key={milestone.id} className="border-b border-navy-700 dark:border-navy-700 light:border-gray-200 hover:bg-navy-700/30 dark:hover:bg-navy-700/30 light:hover:bg-gray-100">
                  <td className="py-3 px-4 text-white dark:text-white light:text-gray-900 font-mono text-sm">{milestone.id}</td>
                  <td className="py-3 px-4 text-white dark:text-white light:text-gray-900 text-sm">{milestone.amount}</td>
                  <td className="py-3 px-4">
                    <span className={`px-2 py-1 rounded text-xs font-semibold ${getStatusColor(milestone.status)}`}>
                      {milestone.status}
                    </span>
                  </td>
                  <td className="py-3 px-4">
                    {milestone.proof_uri ? (
                      <a
                        href={milestone.proof_uri}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-cyan-400 dark:text-cyan-400 light:text-cyan-600 hover:text-cyan-300 text-sm underline"
                      >
                        View
                      </a>
                    ) : (
                      <span className="text-gray-500 dark:text-gray-500 light:text-gray-600 text-sm">—</span>
                    )}
                  </td>
                  <td className="py-3 px-4">
                    <MilestoneActions
                      milestone={milestone}
                      agreement={agreement}
                      onSuccess={onUpdate}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}

function getStatusColor(status: string): string {
  const colors: Record<string, string> = {
    Pending: 'bg-gray-600 text-white',
    Funded: 'bg-blue-600 text-white',
    WorkSubmitted: 'bg-yellow-600 text-white',
    Completed: 'bg-green-600 text-white',
    Disputed: 'bg-red-600 text-white',
    Refunded: 'bg-gray-500 text-white',
    Cancelled: 'bg-slate-500 text-white',
  }
  return colors[status] || 'bg-gray-600 text-white'
}
