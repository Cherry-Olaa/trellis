import { useEffect, useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { useTheme } from '../context/ThemeContext'
import { CONTRACT_ID } from '../lib/config'
import { ExplorerLink } from './ExplorerLink'
import { NetworkStatus } from './NetworkStatus'
import WalletConnect from './WalletConnect'
import { WalletErrorBoundary } from './WalletErrorBoundary'
import MoonIcon from './icons/MoonIcon'
import SunIcon from './icons/SunIcon'
import MenuIcon from './icons/MenuIcon'
import CloseIcon from './icons/CloseIcon'

const NAV_LINKS = [
  { to: '/', label: 'Home', isActive: (path: string) => path === '/' },
  { to: '/create', label: 'Create Agreement', isActive: (path: string) => path === '/create' },
  {
    to: '/status',
    label: 'Check Status',
    isActive: (path: string) => path.startsWith('/status') || path.startsWith('/agreement'),
  },
  { to: '/history', label: 'History', isActive: (path: string) => path === '/history' },
]

function Navbar() {
  const location = useLocation()
  const { theme, toggleTheme } = useTheme()
  const [mobileOpen, setMobileOpen] = useState(false)

  // Close the mobile menu on navigation so it never lingers over the next page.
  useEffect(() => {
    setMobileOpen(false)
  }, [location.pathname])

  return (
    <nav className="relative bg-navy-900 dark:bg-navy-900 light:bg-white border-b border-navy-700 dark:border-navy-700 light:border-gray-200 px-4 sm:px-6 py-4 flex items-center justify-between">
      <div className="flex items-center gap-6">
        <Link to="/" className="flex items-center gap-3 cursor-pointer hover:opacity-80 transition-opacity">
          <span className="text-cyan-400 text-xl font-bold tracking-tight">Trellis</span>
          <span className="hidden sm:inline text-gray-500 dark:text-gray-500 light:text-gray-600 text-sm">Trustless Milestone Escrow</span>
        </Link>

        <div className="hidden md:flex items-center gap-4 ml-4">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.to}
              to={link.to}
              className={`text-sm font-medium transition-colors ${
                link.isActive(location.pathname)
                  ? 'text-cyan-400'
                  : 'text-gray-400 dark:text-gray-400 light:text-gray-600 hover:text-cyan-400'
              }`}
            >
              {link.label}
            </Link>
          ))}
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-end gap-2 sm:gap-4">
        <NetworkStatus />
        <ExplorerLink
          type="contract"
          value={CONTRACT_ID}
          className="hidden md:inline-flex text-xs"
        />
        <button
          onClick={toggleTheme}
          aria-label={`Switch to ${theme === 'light' ? 'dark' : 'light'} mode`}
          className="flex items-center justify-center w-11 h-11 -my-1 rounded-lg border border-navy-700 dark:border-navy-700 light:border-gray-300 text-gray-400 dark:text-gray-400 light:text-gray-600 hover:bg-navy-800 dark:hover:bg-navy-800 light:hover:bg-gray-100 transition-all"
        >
          {theme === 'light' ? (
            <MoonIcon className="w-5 h-5" />
          ) : (
            <SunIcon className="w-5 h-5" />
          )}
        </button>
        <WalletConnect />

        <button
          type="button"
          onClick={() => setMobileOpen((open) => !open)}
          aria-label={mobileOpen ? 'Close navigation menu' : 'Open navigation menu'}
          aria-expanded={mobileOpen}
          aria-controls="mobile-nav-menu"
          className="md:hidden flex items-center justify-center w-11 h-11 -my-1 rounded-lg border border-navy-700 dark:border-navy-700 light:border-gray-300 text-gray-400 dark:text-gray-400 light:text-gray-600 hover:bg-navy-800 dark:hover:bg-navy-800 light:hover:bg-gray-100 transition-all"
        >
          {mobileOpen ? <CloseIcon className="w-5 h-5" /> : <MenuIcon className="w-5 h-5" />}
        </button>
      </div>

      {mobileOpen && (
        <div
          id="mobile-nav-menu"
          className="md:hidden absolute left-0 right-0 top-full z-20 bg-navy-900 dark:bg-navy-900 light:bg-white border-b border-navy-700 dark:border-navy-700 light:border-gray-200 shadow-lg"
        >
          <div className="flex flex-col px-4 py-2">
            {NAV_LINKS.map((link) => (
              <Link
                key={link.to}
                to={link.to}
                className={`min-h-[44px] flex items-center text-base font-medium border-b border-navy-800 dark:border-navy-800 light:border-gray-100 last:border-b-0 transition-colors ${
                  link.isActive(location.pathname)
                    ? 'text-cyan-400'
                    : 'text-gray-300 dark:text-gray-300 light:text-gray-700'
                }`}
              >
                {link.label}
              </Link>
            ))}
          </div>
        </div>
      )}
    </nav>
  )
}

export default Navbar
