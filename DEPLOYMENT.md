# Trellis — Testnet Deployment

## Live Contract

| Network | Contract ID |
|---|---|
| Stellar Testnet | `CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q` |

### Explorer Links

- **Contract:** https://lab.stellar.org/r/testnet/contract/CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q
- **Deploy TX:** https://stellar.expert/explorer/testnet/tx/8587efcf3c302c878eccca0d6e00fb06cb8e09d535675a10f35f98541835d63f

---

## Network Configuration

| | Value |
|---|---|
| Network | Stellar Testnet |
| RPC URL | `https://soroban-testnet.stellar.org` |
| Network Passphrase | `Test SDF Network ; September 2015` |
| Soroban SDK Version | `22.x` (Protocol 26 compatible) |
| Explorer | https://stellar.expert/explorer/testnet |

---

## Verifying Release Artifacts

Each GitHub Release publishes a `.sha256` checksum and a `.asc` GPG detached
signature alongside every WASM/CLI binary artifact.

1. Download the artifact, its `.sha256` file, and its `.asc` file from the
   [Releases page](https://github.com/Trellis-Ecosystem/trellis/releases).
2. Verify the checksum:
   ```bash
   sha256sum -c trellis-x86_64-unknown-linux-gnu.sha256
   ```
3. Verify the GPG signature (import the maintainers' public key first):
   ```bash
   gpg --verify trellis-x86_64-unknown-linux-gnu.asc trellis-x86_64-unknown-linux-gnu
   ```

---

## Deploying Your Own Instance

### Prerequisites

- Rust stable toolchain
- `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- stellar CLI 26.x+:
  https://developers.stellar.org/docs/tools/cli/install-cli

### Step 1 — Create and fund a testnet identity

```bash
stellar keys generate trellis-deployer --network testnet
stellar keys fund trellis-deployer --network testnet --rpc-url https://soroban-testnet.stellar.org
```

### Step 2 — Build the contract WASM

```bash
cd contracts/trellis_core
cargo rustc --manifest-path=Cargo.toml --crate-type=cdylib --target=wasm32-unknown-unknown --release
```

> **Note:** Use `cargo rustc` with `--target=wasm32-unknown-unknown` rather than
> `stellar contract build` — the latter targets `wasm32v1-none` which requires
> soroban-sdk 22.x+ and a newer toolchain. The manual `cargo rustc` command gives
> you full control over the target.

The compiled WASM will be at:
```
target/wasm32-unknown-unknown/release/trellis_core.wasm
```

### Step 3 — Deploy to testnet

```bash
stellar contract deploy \
  --wasm ../../target/wasm32-unknown-unknown/release/trellis_core.wasm \
  --source trellis-deployer \
  --network testnet
```

This returns a `C...` contract address. Save it — you'll need it for all CLI commands.

### Step 4 — Set environment variables

```bash
export STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
export STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export TRELLIS_CONTRACT_ID="<your contract ID>"
export TRELLIS_SOURCE_KEY="<your stellar identity name>"
```

### Step 5 — Build the CLI

```bash
cd cli/trellis_cli
cargo build --release
```

Binary will be at: `target/release/trellis.exe` (Windows) or `target/release/trellis` (Linux/Mac)

---

## CLI Usage

### Create an agreement

```bash
trellis init \
  --agreement-id <64-char hex string> \
  --payer <payer G... address> \
  --payee <payee G... address> \
  --token <USDC SAC contract address> \
  --resolver <dispute resolver G... address> \
  --milestones "1000,2000"
```

> The `--milestones` flag accepts comma-separated amounts in stroops.
> Multiple milestones: `--milestones "1000,2000,3000"`

### Check agreement status

```bash
trellis status --agreement-id <hex>
```

### Fund a milestone

```bash
trellis lock-funds --agreement-id <hex> --milestone-id 0
```

> Must be signed by the payer identity (`TRELLIS_SOURCE_KEY=trellis-payer`)

### Submit work proof

```bash
trellis submit-work \
  --agreement-id <hex> \
  --milestone-id 0 \
  --proof-uri "ipfs://your-proof-hash"
```

> Must be signed by the payee identity

`--proof-uri` is optional — omit it to mark the milestone submitted without a
proof link. Do not pass an empty string: the contract stores `proof_uri` as an
`Option`, so "no proof" is the absent flag, not `""`.
The web UI follows the same rule: leaving the Proof URI field blank submits
`None`, not an empty string.

### Approve and release payment

```bash
trellis approve-release --agreement-id <hex> --milestone-id 0
```

> Must be signed by the payer identity

### Raise a dispute

```bash
trellis raise-dispute \
  --agreement-id <hex> \
  --milestone-id 0 \
  --caller <payer or payee address>
```

> Can be called by either the payer or the payee

In the web UI, the Dispute button only appears when the connected wallet is the
agreement's payer or payee; any other wallet sees an explanatory note instead.

### Resolve a dispute

```bash
trellis resolve-dispute \
  --agreement-id <hex> \
  --milestone-id 0 \
  --refund-to-payer true
```

> Must be signed by the dispute resolver identity.
> `--refund-to-payer true` returns funds to payer.
> `--refund-to-payer false` releases funds to payee.

### Cancel an unfunded milestone

```bash
trellis cancel-milestone --agreement-id <hex> --milestone-id 0
```

> Must be signed by the payer. Only works on milestones still in `Pending` status.

---

## Testnet Token (USDC)

The USDC Stellar Asset Contract address on testnet:

```
CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
```

To derive it yourself:
```bash
stellar contract id asset \
  --network testnet \
  --asset USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

To get testnet USDC, visit the Stellar testnet faucet:
https://developers.stellar.org/docs/tools/quickstart#friendbot

---

## Verified Test Agreement

A live test agreement was created on testnet during initial deployment verification:

| Field | Value |
|---|---|
| Agreement ID | `0101010101010101010101010101010101010101010101010101010101010101` |
| Payer | `GDTFKGGBI4GGHSZ6ILTTJJTXOA4CIKCT6J2DRC3Q3OT3YMWGB7TBLXMK` |
| Payee | `GCGX32T5547AE57PUIAX5CDQWKHHEQFYOPIGXH7O3CENKDX3FR6U67RY` |
| Milestone 0 | 1000 stroops, status: Pending |

> **Note — testnet state is ephemeral:** Stellar resets testnet ledger state
> periodically (roughly quarterly). When that happens, the contract instance
> above and this agreement ID are wiped, and `trellis status` will return
> `agreement not found`. If the command below fails, see
> [Testnet Reset](#testnet-reset) to redeploy and create fresh test data.

Verify it live:
```bash
trellis status --agreement-id 0101010101010101010101010101010101010101010101010101010101010101
```

---

## Testnet Reset

Stellar's testnet is periodically reset, wiping all deployed contract
instances and stored ledger entries (including every Trellis agreement).
When this happens, the contract ID in [Live Contract](#live-contract) and the
agreement ID in [Verified Test Agreement](#verified-test-agreement) both stop
resolving.

### How to tell testnet was reset

- `trellis status --agreement-id <any known ID>` returns `agreement not found`
  even though the ID was previously valid.
- The contract ID no longer resolves on
  [Stellar Expert](https://stellar.expert/explorer/testnet).

### Recovering

1. Redeploy the contract (Steps 1–3 under
   [Deploying Your Own Instance](#deploying-your-own-instance)) to get a new
   contract ID.
2. Update `TRELLIS_CONTRACT_ID` (CLI) and `VITE_CONTRACT_ID`
   (`frontend/.env`) to the new contract ID.
3. Create a fresh test agreement with `trellis init` (see
   [CLI Usage](#cli-usage)) and note the new agreement ID for testing.
4. Optionally run `scripts/reset-testnet.sh`, which automates steps 1–3: it
   rebuilds and redeploys the contract, builds the CLI, and creates a new
   test agreement, printing the new contract and agreement IDs.

```bash
./scripts/reset-testnet.sh
```

The script requires the same prerequisites as manual deployment (Rust,
`wasm32-unknown-unknown` target, `stellar` CLI) and a funded
`trellis-deployer` identity — it will create and fund one via Friendbot if it
does not already exist.

---

## Verified CLI Commands

| Command | Status | Notes |
|---|---|---|
| `init` | ✅ Verified on testnet | Agreement created on-chain |
| `status` | ✅ Verified on testnet | Returns full agreement state as JSON |
| `lock-funds` | ✅ Verified on testnet | Funds successfully locked in escrow (Issue #7) |
| `submit-work` | ✅ Verified on testnet | Work proof submitted and milestone transitioned to WorkSubmitted (Issue #8) |
| `approve-release` | ✅ Verified on testnet | Funds released to payee, milestone marked Completed (Issue #8) |
| `raise-dispute` | ✅ Verified on testnet | Dispute raised and milestone transitioned to Disputed (Issue #9) |
| `resolve-dispute` | ✅ Verified on testnet (both paths) | Resolves to both Refunded (payer) and Completed (payee) states (Issue #9) |
| `cancel-milestone` | ✅ Implemented | Cancels unfunded milestones |

---

---

## End-to-End Test Flows (Verified on Testnet)

### Flow 1: Happy Path — Lock → Submit → Approve (Issues #7 & #8)

**Objective:** Verify the complete payment flow from initial funding through approval and release.

**Steps:**
1. Create agreement with payer, payee, resolver identities
2. Fund milestone 0 (payer calls `lock-funds`)
3. Submit work proof (payee calls `submit-work`)
4. Approve and release (payer calls `approve-release`)

**Verification:**
- ✅ Milestone transitions: `Pending` → `Funded` → `WorkSubmitted` → `Completed`
- ✅ Token transfer confirmed from payer to contract, then to payee
- ✅ Events emitted at each stage (queryable via event filters)
- ✅ `status` returns `Completed` state with proof URI recorded

### Flow 2: Dispute Path — Raise → Resolve as Refund (Issue #9)

**Objective:** Verify dispute resolution when resolver rules in payer's favor.

**Steps:**
1. Create and fund agreement as above
2. Dispute raised by payee (payer calls `raise-dispute` with payee caller)
3. Resolver ruling: refund to payer (resolver calls `resolve-dispute --refund-to-payer true`)

**Verification:**
- ✅ Milestone transitions: `Funded` → `Disputed` → `Refunded`
- ✅ Funds returned to payer's address
- ✅ Payee does not receive payment
- ✅ Events show dispute resolution with refund outcome

### Flow 3: Dispute Path — Raise → Resolve as Release (Issue #9)

**Objective:** Verify dispute resolution when resolver rules in payee's favor.

**Steps:**
1. Create and fund agreement
2. Work submitted, then dispute raised by payer (payer calls `raise-dispute` with caller=payer)
3. Resolver ruling: release to payee (resolver calls `resolve-dispute --refund-to-payer false`)

**Verification:**
- ✅ Milestone transitions: `WorkSubmitted` → `Disputed` → `Completed`
- ✅ Funds released to payee's address
- ✅ Payer does not recover funds
- ✅ Events show dispute resolution with payee-win outcome

---

## Frontend SPA Routing

The frontend is a single-page app using client-side routing (React Router). A
direct load or refresh on a nested route (e.g. `/agreement/123`) must be
served `index.html` by the web server, not a 404 — the router only resolves
the path once the JS bundle loads.

- **Vite dev server:** already configured via `appType: 'spa'` in
  `frontend/vite.config.ts`; no extra setup needed.
- **Netlify / Vercel / other static hosts that honor `_redirects`:**
  `frontend/public/_redirects` ships a catch-all
  (`/*  /index.html  200`) that is copied into `dist/` on build.
- **nginx:** add a fallback to the `location /` block serving the built
  `dist/` directory:

  ```nginx
  location / {
      try_files $uri $uri/ /index.html;
  }
  ```

- **Apache:** enable `mod_rewrite` and add to `dist/.htaccess`:

  ```apache
  RewriteEngine On
  RewriteBase /
  RewriteRule ^index\.html$ - [L]
  RewriteCond %{REQUEST_FILENAME} !-f
  RewriteCond %{REQUEST_FILENAME} !-d
  RewriteRule . /index.html [L]
  ```

---

## Known Issues & Notes

- **Auth per command:** Each CLI command must be signed by the correct identity.
  Change `TRELLIS_SOURCE_KEY` to match the required signer per operation
  (payer for `init`, `lock-funds`, `approve-release`; payee for `submit-work`;
  resolver for `resolve-dispute`).

- **WASM build target:** Use `wasm32-unknown-unknown` via `cargo rustc` rather
  than `stellar contract build` until the project upgrades to a toolchain that
  fully supports `wasm32v1-none`.

- **Windows paths:** On Windows Git Bash, use the full path to the binary:
  `C:\Users\<you>\Documents\Trellis\target\release\trellis.exe`

- **Native RPC client (#11):** The current implementation falls back to the stellar CLI
  for transaction signing and submission. A full native Rust implementation would require
  integrating the Stellar SDK for key management, transaction envelope building, and
  signing. The structure is in place for future native implementation in `cli/trellis_cli/src/rpc.rs`.
