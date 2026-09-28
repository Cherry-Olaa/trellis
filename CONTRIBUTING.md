# Contributing to Trellis

Welcome to Trellis. This guide is the single source of truth for setting up the repository, running the baseline checks, claiming issues, and opening a reviewable pull request.

## 1. Prerequisites

Install and verify these tools before changing code:

- Rust stable toolchain, installed through `rustup`.
- The `wasm32-unknown-unknown` target for Soroban contract builds.
- `stellar` CLI 26.x or newer. Install docs: <https://developers.stellar.org/docs/tools/cli/install-cli>.
- Git.

Verify Rust and Git:

```bash
rustc --version
cargo --version
git --version
```

Install the Soroban WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

Verify the Stellar CLI after installing it:

```bash
stellar --version
```

## 2. Fork and Clone

Fork `Trellis-Ecosystem/trellis` on GitHub, then clone your fork locally:

```bash
git clone https://github.com/<your-github-user>/trellis.git
cd trellis
```

Add the upstream remote so you can sync with the canonical repository:

```bash
git remote add upstream https://github.com/Trellis-Ecosystem/trellis.git
git remote -v
```

Before starting any task, update your local branch:

```bash
git fetch upstream
git checkout main
git merge upstream/main
```

## 3. Project Structure

Trellis is a Rust workspace with a Soroban contract, a CLI, and a frontend scaffold.

```text
trellis/
├── contracts/trellis_core/  # Soroban smart contract crate
├── cli/trellis_cli/         # Rust CLI binary for contract workflows
├── frontend/                # React/Vite frontend scaffold
├── Cargo.toml               # Workspace manifest
├── DEPLOYMENT.md            # Testnet deployment and verification guide
└── README.md                # Product overview and quickstart
```

### `contracts/trellis_core`

This crate contains the escrow contract compiled to WASM for Soroban.

Files in `contracts/trellis_core/src/`:

- `types.rs` — shared contract data types, including `Agreement`, `Milestone`, and `EscrowStatus`.
- `storage.rs` — ledger storage helpers and `DataKey` access patterns.
- `errors.rs` — `TrellisError` variants returned by contract entrypoints.
- `events.rs` — event emission helpers used by off-chain consumers.
- `lib.rs` — the `#[contractimpl]` entrypoints and state-transition logic.
- `test.rs` — Soroban sandbox integration tests for the core contract lifecycle.

### `cli/trellis_cli`

This crate builds the `trellis` command-line binary. It wraps the Stellar CLI/Soroban RPC flow and exposes user-facing commands such as `init`, `lock-funds`, `submit-work`, `approve-release`, `raise-dispute`, `resolve-dispute`, `cancel-milestone`, and `status`.

Important files:

- `src/main.rs` — `clap` command parser and top-level dispatch.
- `src/config.rs` — environment-driven RPC, network, contract, and source-account configuration.
- `src/rpc.rs` — shell-out invocation layer and `InvokeOutput` handling.
- `src/commands/mod.rs` — command implementations.
- `src/utils.rs` — small CLI utility helpers.

## 4. Pre-commit Hooks

Trellis ships pre-commit hooks in `.githooks/` that catch lint errors, formatting issues, and accidentally committed secrets before they reach CI.  The hooks are not installed automatically — run the setup command once after cloning:

```bash
./scripts/install-hooks.sh
```

This is equivalent to:

```bash
git config core.hooksPath .githooks
```

### What the hook checks

| Check | Scope | Triggered when |
|---|---|---|
| `cargo fmt --check` | Rust workspace | Any `.rs` file is staged |
| `cargo clippy -D warnings` | Rust workspace | Any `.rs` file is staged |
| `npm run lint` (oxlint) | `frontend/` | Any `.ts`/`.tsx`/`.js`/`.jsx` file is staged |
| `npm run typecheck` (tsc) | `frontend/` | Any `.ts`/`.tsx`/`.js`/`.jsx` file is staged |
| Secret scan | All staged files | Every commit |

The secret scan uses **gitleaks** if it is installed, otherwise falls back to a basic regex scan.  Install gitleaks for full coverage:

```bash
# macOS
brew install gitleaks

# Linux (replace VERSION with latest from https://github.com/gitleaks/gitleaks/releases)
VERSION=8.27.2
curl -sSfL "https://github.com/gitleaks/gitleaks/releases/download/v${VERSION}/gitleaks_${VERSION}_linux_x64.tar.gz" \
  | tar -xz -C /usr/local/bin gitleaks
```

### Bypassing the hook

Use bypass flags sparingly — CI will still catch the same issues:

```bash
# Skip all checks (emergency only)
SKIP_HOOKS=1 git commit -m "..."

# Skip individual checks
SKIP_RUST_FMT=1   git commit ...   # skip cargo fmt
SKIP_RUST_LINT=1  git commit ...   # skip cargo clippy
SKIP_JS_LINT=1    git commit ...   # skip oxlint
SKIP_TYPECHECK=1  git commit ...   # skip tsc
SKIP_SECRETS=1    git commit ...   # skip secret scan
```

### Allowlisting gitleaks false positives

If gitleaks flags a false positive, add a `# gitleaks:allow` comment on the triggering line, or add a per-rule allowance in `.gitleaks.toml` at the repository root with a justification comment.

## 5. Static Analysis with Semgrep

Before committing contract code, run Semgrep to detect common Soroban vulnerability patterns:

```bash
semgrep --config .semgrep/ contracts/
```

Install Semgrep if not already available:

```bash
brew install semgrep   # macOS
pip install semgrep    # Linux/Windows with Python
```

Fix any warnings before opening a PR. The CI will automatically scan all pull requests.

## 6. Running the Contract Tests

Run the contract test suite before changing contract code:

```bash
cd contracts/trellis_core
cargo test
```

The suite currently runs **43 tests**, split across three modules:

| Module | Tests | Coverage |
| --- | --- | --- |
| `src/test.rs` | 23 | Example-based lifecycle, error paths, role checks, and TTL extension |
| `src/test_properties.rs` | 11 | `proptest` invariants — balance conservation, invalid amounts, and milestone isolation |
| `src/test_panic_boundaries.rs` | 9 | Panic-boundary and fuzz coverage for every entrypoint |
| **Total** | **43** | |

Representative example-based tests in `src/test.rs` include `test_happy_path`, `test_double_init_fails`, `test_dispute_and_refund_to_payer`, `test_cancel_unfunded_milestone`, `test_cancel_funded_milestone_fails_with_invalid_state_transition`, `test_get_agreement`, `test_get_milestone_unknown_agreement_returns_error`, `test_batch_lock_funds_partial_failure`, and the six `*_wrong_role_fails` authorization tests.

![Contract tests](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Trellis-Ecosystem/trellis/master/.github/badges/contract-tests.json)

The badge and the counts above are refreshed by the [`test-count-badge`](.github/workflows/test-count-badge.yml) workflow on every push to `master`, so the documented total always tracks `cargo test`.

If any baseline test fails, open an issue before continuing. Do not start feature, bug-fix, or documentation work on a broken baseline unless your assigned issue is specifically about that failure.

After running tests from inside `contracts/trellis_core`, return to the workspace root when you are done:

```bash
cd ../..
```

## 7. Building the CLI

Build the CLI from its crate directory:

```bash
cd cli/trellis_cli
cargo build --release
```

The compiled binary lands in the workspace root target directory, not inside the CLI crate:

- macOS/Linux: `target/release/trellis`
- Windows: `target/release/trellis.exe`

It does **not** land at `cli/trellis_cli/target/release/`.

Return to the workspace root after the build:

```bash
cd ../..
```

## 8. Understanding the State Machine

Read this before touching contract code. Trellis models each milestone as a state machine:

```text
Pending ──lock_funds──► Funded ──submit_work──► WorkSubmitted ──approve_and_release──► Completed
   │                       │                          │
   │                  raise_dispute              raise_dispute
   │                       └──────────────────────────┘
   │                                    │
cancel_unfunded                         ▼
   │                                Disputed
   │                            ┌───────┴────────┐
   │                            │                 │
   ▼                            ▼                 ▼
Cancelled              resolve_dispute      resolve_dispute
                        (refund payer)      (release payee)
                              │                   │
                              ▼                   ▼
                          Refunded            Completed
```

Transitions and entrypoints:

- `Pending -> Funded` is triggered by `lock_funds`. The payer deposits the milestone amount into the contract.
- `Pending -> Cancelled` is triggered by `cancel_unfunded_milestone`. The payer cancels a milestone that has never been funded.
- `Funded -> WorkSubmitted` is triggered by `submit_work`. The payee attaches proof of completed work.
- `Funded -> Disputed` is triggered by `raise_dispute`. Either payer or payee can request resolver review before work is submitted.
- `WorkSubmitted -> Completed` is triggered by `approve_and_release`. The payer accepts the work and funds are released to the payee.
- `WorkSubmitted -> Disputed` is triggered by `raise_dispute`. Either side can escalate submitted work for resolver review.
- `Disputed -> Refunded` is triggered by `resolve_dispute` when the resolver rules for the payer.
- `Disputed -> Completed` is triggered by `resolve_dispute` when the resolver rules for the payee.
- `get_agreement` is read-only. It does not transition state; it returns the current agreement snapshot.
- `init` creates the agreement and starts each milestone in `Pending`.

Never add a new transition or bypass an existing state without first discussing it in the linked issue.

## 9. How to Claim an Issue

1. Browse open issues and look for `good first issue` or `help wanted` labels.
2. Comment exactly: `I'd like to work on this`.
3. Wait for maintainer confirmation before opening a PR.
4. Do not open a PR for an issue nobody has confirmed you can work on.
5. If an issue has been claimed but shows no activity after its stated timeframe, comment asking whether it is still being worked on.

This avoids duplicated effort and keeps maintainers from reviewing competing solutions for the same small task.

## 10. Branch Naming

Use short, scoped branch names:

- `feat/short-description` for features.
- `fix/short-description` for bug fixes.
- `docs/short-description` for documentation.
- `test/short-description` for testnet verification tasks.

Examples:

```bash
git checkout -b feat/agreement-status-page
git checkout -b fix/dispute-resolution-edge-case
git checkout -b docs/contributing-guide
git checkout -b test/live-status-command
```

## 11. Review Ownership and CODEOWNERS

The repository uses a [CODEOWNERS](.github/CODEOWNERS) file so that pull requests are automatically routed to the maintainers responsible for the code you touched. Review requests are issued when the PR is opened and re-evaluated on every push.

| Area | Pattern | Owners |
| --- | --- | --- |
| Everything else (default fallback) | `*` | @ALLEN-AYODEJI |
| Smart contracts | `/contracts/`, root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | @ALLEN-AYODEJI |
| CLI | `/cli/` | @ALLEN-AYODEJI |
| Frontend | `/frontend/` | @ALLEN-AYODEJI, @Folex1275 |
| Scripts and tooling | `/scripts/`, `/Makefile` | @ALLEN-AYODEJI |
| CI and GitHub configuration | `/.github/` | @ALLEN-AYODEJI, @Oryke, @Felamsy |
| Root documentation | `/*.md` | @ALLEN-AYODEJI |

How matching works:

- A PR that touches files in several areas requests a review from the owners of every matching area. A PR changing both `contracts/trellis_core/src/lib.rs` and `frontend/src/App.tsx` is assigned to the contract owner and the frontend owner.
- When several patterns match the same file, the last matching rule in the file wins. Specific rules are therefore listed after broader ones.
- CODEOWNERS only *requests* reviews. Whether an owner's approval is mandatory is controlled by branch protection ("Require review from Code Owners") on the default branch.

Rules for changing ownership:

- Owners must have write access to this repository. A username or team without access is silently ignored by GitHub, and no review request is sent.
- To become an owner for an area, or to change an assignment, open an issue describing your experience with that area. Do not edit `.github/CODEOWNERS` directly without agreement.

## 12. PR Requirements

All of the following must be true before requesting review:

- `cargo test` passes 41/41 in `contracts/trellis_core`.
- `cargo build` passes with zero warnings in both Rust crates you touched.
- The PR description explains what changed and why.
- The PR references the issue number using `Closes #X`.
- No files outside the linked issue's scope are changed.
- No changes are made to `contracts/trellis_core` unless the issue explicitly requires contract changes.
- No new dependencies are added without prior discussion in the issue thread.
- **If you changed contract code**: regenerate test snapshots and commit them (see [Test Snapshots](#13-test-snapshots) below).

Suggested final checks from the workspace root:

```bash
cargo test
cargo build --workspace
```

For contract-specific work, also run:

```bash
cd contracts/trellis_core
cargo test
cd ../..
```

For CLI-specific work, also run:

```bash
cd cli/trellis_cli
cargo build --release
cd ../..
```

## 13. Test Snapshots

The Soroban test framework records ledger state at each test step into JSON files under `contracts/trellis_core/test_snapshots/`. These files are committed to the repository so reviewers can see exactly what state the contract produces for every test case.

**Why this matters:** If you change contract logic or add/remove entrypoints, the snapshot files will diverge from what the tests actually produce. A PR with stale snapshots will fail the CI snapshot-validation step, even if `cargo test` itself passes.

### Regenerating snapshots

Whenever you change contract code, regenerate the snapshots before opening a PR:

```bash
make test-snapshots-update
```

This is equivalent to:

```bash
SOROBAN_TEST_SNAPSHOT_FILE=overwrite cargo test --manifest-path contracts/trellis_core/Cargo.toml
```

After regenerating, review the diff:

```bash
git diff contracts/trellis_core/test_snapshots/
```

If the diff looks correct (ledger entries reflecting your intentional change), stage and commit the updated snapshot files as part of the same commit or PR that contains the contract change.

### What the CI check does

After the regular `cargo test` step, CI runs:

```bash
SOROBAN_TEST_SNAPSHOT_FILE=overwrite cargo test --manifest-path contracts/trellis_core/Cargo.toml
git diff --exit-code contracts/trellis_core/test_snapshots/
```

If any snapshot file differs from what is committed, the build fails with an error message directing you to run `make test-snapshots-update`.

### Rules

- Never manually edit snapshot JSON files. They are generated automatically.
- Do not add `test_snapshots/` to `.gitignore`. The files must be tracked.
- If you add a new test, its snapshot file will be created by `make test-snapshots-update` and must be committed.
- If you delete a test, delete its snapshot file from the repository in the same PR.

## 14. Code Style

Follow the existing patterns in the file you edit. Do not introduce a new style in the same PR.

Rules:

- No commented-out code in PRs.
- No leftover `println!` debug statements in committed code.
- Contract code never uses `panic!`; return a `TrellisError` variant instead.
- CLI code follows the existing shell-out plus `InvokeOutput` pattern in `rpc.rs` unless the issue specifically asks you to change it.
- Keep Soroban contract code compatible with `#![no_std]` expectations.
- Prefer small, reviewable PRs over broad refactors.

Before committing Rust changes, format them:

```bash
cargo fmt
```

## 15. Frontend Versioning

The frontend (`frontend/package.json`) follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes to the contract interface the frontend depends on, or a rewrite of core user flows.
- **MINOR** — new user-facing features that stay backward compatible (e.g. a new page, a new wallet capability).
- **PATCH** — bug fixes, accessibility fixes, styling, and other non-feature changes.

To cut a new version:

```bash
cd frontend
npm version patch   # or minor / major
```

This updates `package.json` and creates a matching git tag. The version is injected into the build via `vite.config.ts` (`__APP_VERSION__`, sourced from `npm_package_version`) and rendered in the app footer, so every deployed build is traceable to a version.

## 16. Frontend Tests and Manual Testnet Verification

Run the frontend checks before opening a PR that touches `frontend/`:

```bash
cd frontend
npm ci
npm run lint
npm run typecheck
npm run test
npm run build
```

For changes that affect contract calls or wallet flows, also verify manually against Testnet:

1. Follow the deployment/setup steps in [DEPLOYMENT.md](DEPLOYMENT.md) to point the frontend at a Testnet contract instance.
2. Run `npm run dev` and exercise the affected flow end to end (e.g. init, fund, submit, approve/dispute) using a Freighter Testnet account.
3. Confirm the transaction succeeds in Freighter and the resulting state is reflected in the UI.

## 17. Getting Help

Use the right channel for the question:

- Open a GitHub Discussion for general questions about Trellis, Soroban, or contributor workflow.
- Comment directly on the issue you are working on for issue-specific questions.
- Tag the maintainer if you are blocked for more than 24 hours.

When asking for help, include:

- Your operating system.
- The command you ran.
- The full error output.
- The branch and issue number.
- What you already tried.
