import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render, act } from '@testing-library/react'
import { WalletProvider, useWallet } from './WalletContext'

vi.mock('../lib/wallet', () => ({
  isFreighterInstalled: vi.fn().mockResolvedValue(false),
  connectWallet: vi.fn(),
  getPublicKey: vi.fn().mockResolvedValue('GABC1234'),
  getNetworkPassphrase: vi.fn().mockResolvedValue('Test SDF Network ; September 2015'),
  isAppAllowed: vi.fn().mockResolvedValue(false),
}))

vi.mock('../lib/config', () => ({
  NETWORK_PASSPHRASE: 'Test SDF Network ; September 2015',
  CONTRACT_ID: 'CTEST',
  RPC_URL: 'https://rpc.test',
}))

describe('WalletProvider', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('provides wallet context values', () => {
    // `WalletContextValue` is not exported from the module, so the context
    // shape is recovered from the public `useWallet` hook return type. The
    // definite-assignment assertion keeps the test readable: the value is
    // always written by `TestComponent` before each assertion reads it.
    let contextValue!: ReturnType<typeof useWallet>

    function TestComponent() {
      contextValue = useWallet()
      return null
    }

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    )

    expect(contextValue).toBeDefined()
    expect(contextValue).toHaveProperty('status')
    expect(contextValue).toHaveProperty('publicKey')
    expect(contextValue).toHaveProperty('connected')
    expect(contextValue).toHaveProperty('wrongNetwork')
    expect(contextValue).toHaveProperty('error')
    expect(contextValue).toHaveProperty('connect')
    expect(contextValue).toHaveProperty('disconnect')
    expect(contextValue).toHaveProperty('clearError')
    expect(contextValue).toHaveProperty('recheckInstall')
  })

  it('clears polling interval when component unmounts', async () => {
    const clearIntervalSpy = vi.spyOn(globalThis, 'clearInterval')

    const { unmount } = render(
      <WalletProvider>
        <div />
      </WalletProvider>,
    )

    await act(async () => {
      vi.runAllTimersAsync()
    })

    unmount()

    expect(clearIntervalSpy).toHaveBeenCalled()
    clearIntervalSpy.mockRestore()
  })

  it('does not throw when mounted and unmounted rapidly', async () => {
    expect(() => {
      for (let i = 0; i < 5; i++) {
        const { unmount } = render(
          <WalletProvider>
            <div />
          </WalletProvider>,
        )
        unmount()
      }
    }).not.toThrow()
  })

  it('initializes with detecting status', () => {
    // `WalletContextValue` is not exported from the module, so the context
    // shape is recovered from the public `useWallet` hook return type. The
    // definite-assignment assertion keeps the test readable: the value is
    // always written by `TestComponent` before each assertion reads it.
    let contextValue!: ReturnType<typeof useWallet>

    function TestComponent() {
      contextValue = useWallet()
      return null
    }

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    )

    expect(contextValue.status).toBe('detecting')
  })

  it('handles disconnection gracefully', async () => {
    // `WalletContextValue` is not exported from the module, so the context
    // shape is recovered from the public `useWallet` hook return type. The
    // definite-assignment assertion keeps the test readable: the value is
    // always written by `TestComponent` before each assertion reads it.
    let contextValue!: ReturnType<typeof useWallet>

    function TestComponent() {
      contextValue = useWallet()
      return null
    }

    const { rerender } = render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    )

    await act(async () => {
      contextValue.disconnect()
    })

    expect(contextValue.connected).toBe(false)
  })

  it('clears errors with clearError function', async () => {
    // `WalletContextValue` is not exported from the module, so the context
    // shape is recovered from the public `useWallet` hook return type. The
    // definite-assignment assertion keeps the test readable: the value is
    // always written by `TestComponent` before each assertion reads it.
    let contextValue!: ReturnType<typeof useWallet>

    function TestComponent() {
      contextValue = useWallet()
      return null
    }

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    )

    await act(async () => {
      contextValue.clearError()
    })

    expect(contextValue.error).toBeNull()
  })
})
