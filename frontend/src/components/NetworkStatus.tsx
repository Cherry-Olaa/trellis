import { useStellarStatus } from '../hooks/useStellarStatus'
import { ACTIVE_NETWORK, explorerBaseUrl, networkLabel } from '../lib/explorer'

/**
 * Live network badge for the navbar: links to Stellar Expert for the active
 * network and reflects RPC health (via `getHealth`) in its dot and label.
 */
export function NetworkStatus() {
  const { status, latency } = useStellarStatus()
  const network = networkLabel(ACTIVE_NETWORK)

  const config = {
    checking: {
      dot: 'bg-gray-500',
      pulse: '',
      label: network,
      text: 'text-gray-400',
      border: 'border-gray-500/30 hover:border-gray-500/60',
    },
    online: {
      dot: 'bg-emerald-400',
      pulse: 'animate-ping',
      label: network,
      text: 'text-emerald-400',
      border: 'border-emerald-400/30 hover:border-emerald-400/60',
    },
    degraded: {
      dot: 'bg-yellow-400',
      pulse: 'animate-ping',
      label: `${network} · Degraded`,
      text: 'text-yellow-400',
      border: 'border-yellow-400/30 hover:border-yellow-400/60',
    },
    offline: {
      dot: 'bg-red-400',
      pulse: '',
      label: `${network} · Offline`,
      text: 'text-red-400',
      border: 'border-red-400/30 hover:border-red-400/60',
    },
  }

  const { dot, pulse, label, text, border } = config[status]

  return (
    <a
      href={explorerBaseUrl()}
      target="_blank"
      rel="noopener noreferrer"
      title={
        latency !== null
          ? `RPC latency: ${latency}ms — browse the Stellar ${network} on Stellar Expert`
          : `Browse the Stellar ${network} on Stellar Expert`
      }
      aria-label={`Network status: ${network} ${status} — view on Stellar Expert`}
      role="status"
      aria-live="polite"
      className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition-colors hover:bg-white/5 ${border} ${text}`}
    >
      <span className="relative flex items-center justify-center w-2 h-2" aria-hidden="true">
        <span className={`absolute inline-flex h-full w-full rounded-full ${dot} opacity-50 ${pulse}`} />
        <span className={`relative inline-flex rounded-full h-1.5 w-1.5 ${dot}`} />
      </span>
      <span className="hidden sm:inline">
        {label}
        {latency !== null && status === 'online' && (
          <span className="ml-1 font-mono text-gray-500">{latency}ms</span>
        )}
      </span>
    </a>
  )
}

export default NetworkStatus
