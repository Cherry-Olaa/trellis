# CLI Integration Tests (#142)

Shell-based tests that drive the built `trellis` CLI binary against a real
Soroban RPC endpoint — a local devnet by default, or Stellar testnet when
`TRELLIS_E2E_NETWORK=testnet`. This is the only layer that exercises the CLI's
actual argument formatting, JSON parsing, and error handling through the real
`stellar contract invoke` subprocess.

## Prerequisites

- Docker (or another engine `stellar container` supports)
- [`stellar` CLI](https://developers.stellar.org/docs/tools/cli/install-cli) 26.x+ on `PATH`
- Rust with the `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

## Running

```bash
bash tests/cli_integration/run.sh
```

The script is self-contained and idempotent:

1. By default, starts a local Soroban devnet container if one isn't already
   running and waits for it to report healthy. In testnet mode, uses the
   configured testnet RPC endpoint instead.
2. Generates and funds separate `payer`/`payee`/`resolver` identities. Testnet
   mode funds fresh identities from the Stellar testnet friendbot.
3. Builds the `trellis` CLI (release) and the `trellis_core` contract
   (wasm32), then deploys a fresh contract instance and a native-XLM SAC test
   token.
4. Runs the full milestone lifecycle (init → lock-funds → submit-work →
   approve-release), the dispute path (raise-dispute → resolve-dispute), an
   unfunded-milestone cancellation, and key error paths (malformed
   agreement ID, double-init, wrong-signer, `--dry-run`) — asserting on the
   CLI's actual `--json` output.
5. Prints a pass/fail summary and exits non-zero if anything failed.

Each run generates fresh 64-hex-char agreement IDs (suffixed with a
timestamp) and deploys a new contract instance, so re-running against an
already-running container never collides with a previous run's state.

CI runs this automatically (`.github/workflows/contract-ci.yml`, job
`cli-integration`) on every push/PR touching `contracts/` or `cli/`, since it
only needs a local Docker container — no live testnet or credentials
required. The separate manual workflow `.github/workflows/e2e-testnet.yml`
runs this same lifecycle suite against Stellar testnet in an isolated key
store, which it removes when the run finishes.
