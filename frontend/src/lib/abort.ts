/**
 * Creates an AbortSignal that automatically aborts after the specified timeout.
 * Provides a fallback for browsers that don't support AbortSignal.timeout().
 *
 * @param timeoutMs - The timeout in milliseconds
 * @returns An AbortSignal that aborts after the specified timeout
 */
export function createAbortSignalWithTimeout(timeoutMs: number): AbortSignal {
  // Check if AbortSignal.timeout is supported (Chrome 103+, Safari 15.4+, etc.)
  if (typeof AbortSignal !== 'undefined' && 'timeout' in AbortSignal) {
    try {
      return AbortSignal.timeout(timeoutMs)
    } catch {
      // Fallback if native implementation fails
      return createAbortSignalFallback(timeoutMs)
    }
  }

  // Fallback for older browsers
  return createAbortSignalFallback(timeoutMs)
}

/**
 * Fallback implementation using AbortController and setTimeout.
 * Works on all browsers that support AbortController.
 */
function createAbortSignalFallback(timeoutMs: number): AbortSignal {
  const controller = new AbortController()
  const timeoutId = setTimeout(() => {
    controller.abort()
  }, timeoutMs)

  // Store the timeoutId on the signal so it can be cleaned up if needed.
  // The `AbortSignal` type has no `_timeoutId` field, so the extra cleanup
  // handle is carried on a locally-declared intersection type instead of
  // widening the whole signal to `any`.
  type AbortSignalWithTimeoutId = AbortSignal & { _timeoutId?: ReturnType<typeof setTimeout> }

  const signal: AbortSignalWithTimeoutId = controller.signal
  signal._timeoutId = timeoutId

  return signal
}
