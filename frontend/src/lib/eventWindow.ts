import { RPC_URL } from './config'

/**
 * How far back (in ledgers) event queries reach. RPC providers only retain
 * `getEvents` history for a bounded window (commonly ~7 days, ~120k ledgers
 * at ~5s each); a `startLedger` older than that is rejected outright. ~5.8
 * days leaves headroom below that window.
 *
 * Full history beyond RPC retention needs an event-indexing service (#496).
 */
export const EVENT_LOOKBACK_LEDGERS = 100_000

export const RETENTION_WINDOW_MESSAGE =
  'The RPC provider no longer retains events this far back, so only recent activity can be shown.'

/** Returns a `startLedger` inside the provider's recent retention window. */
export async function getRecentStartLedger(signal?: AbortSignal): Promise<number> {
  const response = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getLatestLedger' }),
    signal,
  })

  if (!response.ok) {
    throw new Error(`RPC HTTP ${response.status}`)
  }

  const json = await response.json()

  if (json.error) {
    throw new Error(json.error.message)
  }

  const latest = Number(json.result?.sequence)
  if (!Number.isFinite(latest) || latest < 1) {
    throw new Error('RPC getLatestLedger returned no ledger sequence')
  }

  return Math.max(1, latest - EVENT_LOOKBACK_LEDGERS)
}

/**
 * True when an RPC error says the requested start ledger falls outside the
 * provider's retention window (e.g. "startLedger must be between the oldest
 * ledger: X and the latest ledger: Y").
 */
export function isRetentionWindowError(message: string): boolean {
  return /startLedger must be between|oldest ledger/i.test(message)
}
