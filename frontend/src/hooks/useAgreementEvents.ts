import { useCallback, useEffect, useRef, useState } from 'react'
import { xdr } from '@stellar/stellar-sdk'
import { CONTRACT_ID, RPC_URL } from '../lib/config'
import { hexToBytes } from '../lib/format'
import { getRecentStartLedger, isRetentionWindowError, RETENTION_WINDOW_MESSAGE } from '../lib/eventWindow'
import type { RawEventResponse } from '../lib/soroban'
import { isValidEventResponse } from '../lib/soroban'

export interface AgreementEvent {
  type: string
  ledger: number
  txHash: string
  timestamp: string
  agreementId: string
  milestoneId?: number
  amount?: string
  caller?: string
  payer?: string
  payee?: string
  proofUri?: string
  refundedToPayer?: boolean
  cancelledBy?: string
}

const BASE_POLL_INTERVAL_MS = 10_000 // Base poll interval: 10 seconds
const MAX_POLL_INTERVAL_MS = 60_000  // Cap backoff at 60 seconds
const FETCH_TIMEOUT_MS = 30_000     // Per-request timeout: 30 seconds

/**
 * Fetch and parse contract events for a specific agreement.
 *
 * Uses a recursive setTimeout chain so the next poll only starts after the
 * current fetch completes, preventing concurrent request stacking.  Applies
 * linear backoff (capped at MAX_POLL_INTERVAL_MS) when a fetch fails so a
 * flaky network doesn't hammer the RPC endpoint.
 */
export function useAgreementEvents(agreementId: string | null) {
  const [events, setEvents] = useState<AgreementEvent[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Tracks consecutive failure count for backoff calculation.
  const failureCount = useRef(0)
  // Holds the scheduled timer handle so we can cancel it on unmount.
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  // Signals that the component has unmounted — used to suppress state updates.
  const unmountedRef = useRef(false)

  const fetchEvents = useCallback(
    async (signal: AbortSignal): Promise<boolean> => {
      if (!agreementId) return true // treat no-id as "success" (no-op)

      // Guard state updates for unmounted components
      if (!unmountedRef.current) {
        setIsLoading(true)
        setError(null)
      }

      // Combine the caller's signal with a per-request timeout signal.
      const timeoutId = setTimeout(() => {
        /* The fetch API will reject once the timeout AbortController fires. */
      }, FETCH_TIMEOUT_MS)
      const timeoutController = new AbortController()
      const timeoutHandle = setTimeout(
        () => timeoutController.abort(),
        FETCH_TIMEOUT_MS,
      )

      // Merge two abort signals: unmount signal + timeout signal.
      const mergedSignal = (() => {
        const ac = new AbortController()
        signal.addEventListener('abort', () => ac.abort(), { once: true })
        timeoutController.signal.addEventListener('abort', () => ac.abort(), {
          once: true,
        })
        return ac.signal
      })()

      clearTimeout(timeoutId)

      try {
        // Encode agreement_id topic filter
        const idBytes = Buffer.from(agreementId, 'hex')
        const idScVal = xdr.ScVal.scvBytes(idBytes)
        const idTopic = idScVal.toXDR('base64')

        const startLedger = await getRecentStartLedger(mergedSignal)

        const body = {
          jsonrpc: '2.0',
          id: 1,
          method: 'getEvents',
          params: {
            startLedger,
            filters: [
              {
                type: 'contract',
                contractIds: [CONTRACT_ID],
                topics: [[idTopic]],
              },
            ],
            pagination: { limit: 100 },
          },
        }

        const response = await fetch(RPC_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
          signal: mergedSignal,
        })

        if (!response.ok) {
          throw new Error(`RPC HTTP ${response.status}`)
        }

        const json = await response.json()

        if (json.error) {
          throw new Error(json.error.message)
        }

        const rawEvents: unknown[] = json.result?.events || []

        const parsed: AgreementEvent[] = rawEvents
          .filter(isValidEventResponse)
          .map((e: RawEventResponse) => {
            const eventType = parseEventType(e)
            const eventData = parseEventData(e)

            return {
              type: eventType,
              ledger: e.ledger || 0,
              txHash: e.txHash || '',
              timestamp: e.ledgerClosedAt || new Date().toISOString(),
              agreementId,
              ...eventData,
            }
          })

        // Sort by ledger descending (newest first)
        parsed.sort((a, b) => b.ledger - a.ledger)

        if (!unmountedRef.current) {
          setEvents(parsed)
          setIsLoading(false)
        }

        failureCount.current = 0
        return true
      } catch (err) {
        clearTimeout(timeoutHandle)

        // Abort errors (unmount or timeout) are not user-facing failures.
        if (
          signal.aborted ||
          (err instanceof Error && err.name === 'AbortError')
        ) {
          if (!unmountedRef.current) setIsLoading(false)
          return false
        }

        console.error('[useAgreementEvents] Failed to fetch:', err)

        if (!unmountedRef.current) {
          const message = err instanceof Error ? err.message : 'Failed to fetch events'
          setError(isRetentionWindowError(message) ? RETENTION_WINDOW_MESSAGE : message)
          setIsLoading(false)
        }

        failureCount.current += 1
        return false
      } finally {
        clearTimeout(timeoutHandle)
      }
    },
    [agreementId],
  )

  useEffect(() => {
    unmountedRef.current = false
    failureCount.current = 0

    const controller = new AbortController()

    /**
     * Recursive polling: schedule the next poll only after the current fetch
     * resolves, preventing concurrent in-flight requests.  Back off linearly on
     * repeated failures (each failure adds one base interval, capped at max).
     */
    const schedule = (delayMs: number) => {
      timerRef.current = setTimeout(async () => {
        if (controller.signal.aborted) return
        await fetchEvents(controller.signal)
        if (!controller.signal.aborted) {
          const nextDelay = Math.min(
            BASE_POLL_INTERVAL_MS +
              failureCount.current * BASE_POLL_INTERVAL_MS,
            MAX_POLL_INTERVAL_MS,
          )
          schedule(nextDelay)
        }
      }, delayMs)
    }

    // Fire immediately (0 ms delay), then recurse.
    schedule(0)

    return () => {
      unmountedRef.current = true
      controller.abort()
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current)
        timerRef.current = null
      }
    }
  }, [fetchEvents])

  const refetch = useCallback(() => {
    // Cancel any pending timer and fire immediately.
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
    failureCount.current = 0
    void fetchEvents(new AbortController().signal)
  }, [fetchEvents])

  return { events, isLoading, error, refetch }
}

function parseEventType(event: RawEventResponse): string {
  try {
    const topics = event.topic || []
    if (topics.length > 0 && typeof topics[0] === 'string') {
      const firstTopic = xdr.ScVal.fromXDR(topics[0], 'base64')
      if (firstTopic.switch().name === 'scvSymbol') {
        const symbol = firstTopic.sym().toString()
        // Map abbreviated symbols to human-readable names
        return EVENT_TYPE_MAP[symbol] || symbol
      }
    }
  } catch {
    // Fallback to unknown
  }
  return 'unknown'
}

function parseEventData(event: RawEventResponse): Partial<AgreementEvent> {
  try {
    const value = event.value?.xdr
    if (!value || typeof value !== 'string') {
      return {}
    }

    // Get the event type from topics
    const topics = event.topic || []
    let eventTypeSymbol = 'unknown'
    if (topics.length > 0) {
      try {
        const firstTopic = xdr.ScVal.fromXDR(topics[0], 'base64')
        if (firstTopic.switch().name === 'scvSymbol') {
          eventTypeSymbol = firstTopic.sym().toString()
        }
      } catch {
        // Ignore
      }
    }

    const scVal = xdr.ScVal.fromXDR(value, 'base64')

    // Parse common fields from event data structure
    // This is simplified - actual parsing depends on contract event schema
    return {}
  } catch {
    return {}
  }
}
