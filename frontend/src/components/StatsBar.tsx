import { useContractStats } from '../hooks/useContractStats'

const STATS = [
  { key: 'agreements', label: 'Agreements created' },
  { key: 'milestonesLocked', label: 'Milestones locked' },
] as const

function WarningIcon() {
  return (
    <svg
      role="img"
      aria-label="Data may be stale — RPC temporarily unavailable"
      viewBox="0 0 20 20"
      fill="currentColor"
      className="w-4 h-4 text-yellow-400"
    >
      <path
        fillRule="evenodd"
        d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 6a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 6zm0 9a1 1 0 100-2 1 1 0 000 2z"
        clipRule="evenodd"
      />
    </svg>
  )
}

/**
 * StatsBar — live contract-wide counters sourced from on-chain events via
 * useContractStats.
 *
 * - loading: skeleton placeholders, never a fabricated number.
 * - ok:      live counts.
 * - stale:   last known counts plus a warning icon (a refresh failed).
 * - error:   N/A plus an "RPC Unavailable" alert (#93) — never zeros.
 *
 * Counts cover the RPC node's event retention window, not all-time history.
 */
export function StatsBar() {
  const { stats, status, lastUpdated } = useContractStats()

  return (
    <section aria-label="Contract activity" className="w-full max-w-2xl">
      <div className="grid grid-cols-2 gap-4">
        {STATS.map(({ key, label }) => (
          <div
            key={key}
            className="bg-navy-800/50 dark:bg-navy-800/50 light:bg-gray-50 border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg px-6 py-4"
          >
            <div className="flex items-center justify-center gap-2 min-h-[2.25rem]">
              {status === 'loading' ? (
                <span className="h-7 w-12 rounded bg-navy-700 dark:bg-navy-700 light:bg-gray-200 animate-pulse" />
              ) : stats === null ? (
                <span className="text-2xl font-bold text-gray-500">N/A</span>
              ) : (
                <>
                  <span className="text-2xl font-bold font-mono text-white dark:text-white light:text-gray-900">
                    {stats[key]}
                  </span>
                  {status === 'stale' && <WarningIcon />}
                </>
              )}
            </div>
            <p className="mt-1 text-sm text-gray-400 dark:text-gray-400 light:text-gray-600">{label}</p>
          </div>
        ))}
      </div>

      {status === 'error' && (
        <p role="alert" className="mt-3 text-sm text-red-400">
          RPC Unavailable — contract stats could not be loaded.
        </p>
      )}

      {lastUpdated && status !== 'error' && (
        <p className="mt-3 text-xs text-gray-500">
          Recent on-chain activity · updated{' '}
          <time dateTime={lastUpdated}>{new Date(lastUpdated).toLocaleTimeString()}</time>
        </p>
      )}
    </section>
  )
}

export default StatsBar
