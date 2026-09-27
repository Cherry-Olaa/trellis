import { useNavigate, Link } from 'react-router-dom'
import { useTypingAnimation } from '../hooks/useTypingAnimation'
import { HowItWorks } from '../components/HowItWorks'
import { StatsBar } from '../components/StatsBar'
import { useToastActions } from '../hooks/useToast'

/**
 * HomePage — landing page for Trellis.
 *
 * Changes:
 * - Uses useTypingAnimation for the hero heading (#94, #97).
 * - CTA buttons have explicit onClick handlers, cursor-pointer, and active
 *   states so they never appear decorative (#96).
 * - HowItWorks section is extracted to its own component with separate SVG
 *   icon files (#95).
 * - StatsBar shows live contract activity counts from useContractStats (#429).
 */
export default function HomePage() {
  const navigate = useNavigate()
  const toast = useToastActions()

  // Animate the hero heading. The hook handles race conditions and batches
  // character appends to keep re-renders low.
  const heading = useTypingAnimation('Trustless Escrow for Remote Work', 40, 3)

  function handleCreateAgreement() {
    navigate('/create')
  }

  function handleCheckStatus() {
    navigate('/status')
  }

  // Called when a feature-card action is not yet fully wired to a page.
  function handleComingSoon(label: string) {
    toast.info({ title: `${label} — coming soon` })
  }

  return (
    <div className="relative min-h-screen text-gray-200 dark:text-gray-200 light:text-gray-900">
      <div className="relative z-10">
        <main className="flex flex-col items-center justify-center px-6 pt-24 pb-32 text-center">
          {/*
           * aria-label provides the full text immediately to screen readers so
           * they do not read out a partially-typed heading.
           */}
          <h1
            aria-label="Trustless Escrow for Remote Work"
            className="text-4xl sm:text-5xl font-bold tracking-tight text-white dark:text-white light:text-gray-900 max-w-2xl leading-tight min-h-[3rem]"
          >
            {heading}
            {/* Blinking cursor while typing is in progress */}
            {heading.length < 'Trustless Escrow for Remote Work'.length && (
              <span className="animate-pulse" aria-hidden="true">|</span>
            )}
          </h1>

          <p className="mt-4 text-gray-400 dark:text-gray-400 light:text-gray-600 text-lg sm:text-xl max-w-xl">
            Built on Stellar's Soroban smart contract platform
          </p>

          {/* ── CTA Buttons (#96) ───────────────────────────────────────────
            * Both buttons have:
            *   • explicit onClick handlers
            *   • cursor-pointer so the pointer is never absent
            *   • active: scale-95 for tactile press feedback
            *   • focus-visible ring for keyboard users
            */}
          <div className="mt-10 flex flex-col sm:flex-row gap-4">
            <button
              type="button"
              onClick={handleCreateAgreement}
              className="
                cursor-pointer
                bg-cyan-400 text-navy-900 font-semibold px-8 py-3 rounded-lg text-base
                hover:bg-cyan-300
                active:scale-95
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-navy-900
                transition-all
              "
            >
              Create Agreement
            </button>
            <button
              type="button"
              onClick={handleCheckStatus}
              className="
                cursor-pointer
                border border-cyan-400 text-cyan-400 font-semibold px-8 py-3 rounded-lg text-base
                hover:bg-cyan-400/10
                active:scale-95
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-navy-900
                transition-all
              "
            >
              Check Status
            </button>
          </div>

          {/* Live contract activity (#429) */}
          <div className="mt-16 w-full flex justify-center">
            <StatsBar />
          </div>

          {/* Features Section */}
          <div className="mt-20 max-w-4xl grid grid-cols-1 md:grid-cols-3 gap-8">
            <Link
              to="/status"
              className="
                cursor-pointer text-left
                bg-navy-800/50 dark:bg-navy-800/50 light:bg-gray-50 border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6
                hover:border-cyan-400/40 hover:bg-navy-800/70 dark:hover:bg-navy-800/70 light:hover:bg-gray-100
                active:scale-95
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-navy-900
                transition-all
              "
              aria-label="Read any agreement — navigate to status page"
            >
              <h3 className="text-cyan-400 font-bold text-lg mb-2">Read Any Agreement</h3>
              <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm">
                Paste an Agreement ID to view its current on-chain state — no wallet
                connection needed.
              </p>
            </Link>

            <Link
              to="/create"
              className="
                cursor-pointer text-left
                bg-navy-800/50 dark:bg-navy-800/50 light:bg-gray-50 border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6
                hover:border-cyan-400/40 hover:bg-navy-800/70 dark:hover:bg-navy-800/70 light:hover:bg-gray-100
                active:scale-95
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-navy-900
                transition-all
              "
              aria-label="Create and fund milestones — navigate to create page"
            >
              <h3 className="text-cyan-400 font-bold text-lg mb-2">Create &amp; Fund Milestones</h3>
              <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm">
                Create escrow agreements with multiple milestones and fund them directly
                from the browser.
              </p>
            </Link>

            <button
              type="button"
              onClick={() => handleComingSoon('Audit Trail')}
              className="
                cursor-pointer text-left
                bg-navy-800/50 dark:bg-navy-800/50 light:bg-gray-50 border border-navy-700 dark:border-navy-700 light:border-gray-200 rounded-lg p-6
                hover:border-cyan-400/40 hover:bg-navy-800/70 dark:hover:bg-navy-800/70 light:hover:bg-gray-100
                active:scale-95
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-navy-900
                transition-all
              "
              aria-label="Complete audit trail — coming soon"
            >
              <h3 className="text-cyan-400 font-bold text-lg mb-2">Complete Audit Trail</h3>
              <p className="text-gray-400 dark:text-gray-400 light:text-gray-600 text-sm">
                Every action emits an on-chain event — full history and transparency for
                all parties.
              </p>
            </button>
          </div>

          {/* How It Works — extracted component (#95) */}
          <HowItWorks />
        </main>
      </div>
    </div>
  )
}
