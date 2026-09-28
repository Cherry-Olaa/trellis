#!/usr/bin/env bash
# CLI + contract integration test suite (#142).
#
# Deploys the trellis_core contract to a local Soroban devnet (via
# `stellar container start local`) and drives it exclusively through the
# built `trellis` CLI binary, asserting on real on-chain results. This is the
# only layer of testing that exercises the CLI's actual argument formatting
# against the real `stellar contract invoke` CLI and a real RPC endpoint —
# the Rust unit tests in cli/trellis_cli/src never shell out.
#
# Requires: docker, the `stellar` CLI (26.x+) on PATH, and a
# wasm32-unknown-unknown Rust target. See README.md in this directory.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI_BIN="$REPO_ROOT/target/release/trellis"
WASM_PATH="$REPO_ROOT/target/wasm32-unknown-unknown/release/trellis_core.wasm"
RPC_URL="http://localhost:8000/soroban/rpc"
PASSPHRASE="Standalone Network ; February 2017"

PASS_COUNT=0
FAIL_COUNT=0
FAILURES=()

log() { echo "[cli-integration] $*"; }

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  echo "  PASS: $1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  FAILURES+=("$1")
  echo "  FAIL: $1"
}

# assert_eq DESCRIPTION EXPECTED ACTUAL
assert_eq() {
  if [ "$2" = "$3" ]; then
    pass "$1"
  else
    fail "$1 (expected '$2', got '$3')"
  fi
}

# assert_contains DESCRIPTION HAYSTACK NEEDLE
assert_contains() {
  if [[ "$2" == *"$3"* ]]; then
    pass "$1"
  else
    fail "$1 (expected output to contain '$3', got: $2)"
  fi
}

# ---------------------------------------------------------------------------
# Setup: local devnet, identities, build, deploy
# ---------------------------------------------------------------------------

wait_for_healthy() {
  for _ in $(seq 1 30); do
    resp=$(curl -s -X POST "$RPC_URL" -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' || true)
    if echo "$resp" | grep -q '"status":"healthy"'; then
      return 0
    fi
    sleep 2
  done
  log "local network never became healthy"
  return 1
}

if ! curl -s -X POST "$RPC_URL" -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q '"status":"healthy"'; then
  log "starting local Soroban devnet container..."
  stellar container start local
  wait_for_healthy
else
  log "local Soroban devnet already running"
fi

for identity in payer payee resolver; do
  if ! stellar keys address "$identity" >/dev/null 2>&1; then
    stellar keys generate "$identity" --network local --fund >/dev/null
  else
    # Key already exists locally from a previous run, but its account may
    # never have been funded (or the container was recreated since) — fund
    # it again so `init` doesn't fail with "Account not found".
    stellar keys fund "$identity" --network local >/dev/null 2>&1 || true
  fi
done

PAYER=$(stellar keys address payer)
PAYEE=$(stellar keys address payee)
RESOLVER=$(stellar keys address resolver)

log "building trellis CLI (release)..."
cargo build --release --manifest-path "$REPO_ROOT/cli/trellis_cli/Cargo.toml" >/dev/null

log "building trellis_core contract (wasm32)..."
cargo rustc --manifest-path "$REPO_ROOT/contracts/trellis_core/Cargo.toml" \
  --crate-type=cdylib --target=wasm32-unknown-unknown --release >/dev/null

log "resolving test token (native XLM SAC)..."
TOKEN=$(stellar contract id asset --asset native --network local 2>/dev/null | tail -1)
if ! stellar contract info interface --contract-id "$TOKEN" --network local >/dev/null 2>&1; then
  stellar contract asset deploy --asset native --source payer --network local >/dev/null 2>&1
fi

log "deploying trellis_core contract..."
CONTRACT_ID=$(stellar contract deploy --wasm "$WASM_PATH" --source payer --network local 2>/dev/null | tail -1)

log "token=$TOKEN contract=$CONTRACT_ID"

export STELLAR_RPC_URL="$RPC_URL"
export STELLAR_NETWORK_PASSPHRASE="$PASSPHRASE"
export TRELLIS_CONTRACT_ID="$CONTRACT_ID"

# Unique-ish 64-hex-char agreement IDs so repeat runs against the same
# container never collide with a prior run's state.
RUN_ID=$(printf '%08x' "$(date +%s)")
agreement_id() { printf '%056d%s' "$1" "$RUN_ID"; }

# ---------------------------------------------------------------------------
# Test cases — each drives the CLI binary directly, never the raw stellar CLI.
# ---------------------------------------------------------------------------

test_full_lifecycle() {
  local id amount1=1000 amount2=2000
  id=$(agreement_id 1)

  local out
  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json init \
    --agreement-id "$id" --payer "$PAYER" --payee "$PAYEE" \
    --token "$TOKEN" --resolver "$RESOLVER" \
    --milestones "$amount1,$amount2" --yes 2>&1)
  assert_eq "init: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json status --agreement-id "$id" 2>&1)
  assert_eq "status: payer matches" "$PAYER" "$(echo "$out" | jq -r .result.payer)"
  assert_eq "status: milestone 0 amount" "$amount1" "$(echo "$out" | jq -r '.result.milestones[0].amount')"
  assert_eq "status: milestone 0 pending" "Pending" "$(echo "$out" | jq -r '.result.milestones[0].status')"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json lock-funds --agreement-id "$id" --milestone-id 0 --yes 2>&1)
  assert_eq "lock-funds: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: funded after lock" "Funded" "$(echo "$out" | jq -r .result.status)"

  out=$(TRELLIS_SOURCE_KEY=payee "$CLI_BIN" --json submit-work --agreement-id "$id" --milestone-id 0 \
    --proof-uri "ipfs://QmIntegrationTest" --yes 2>&1)
  assert_eq "submit-work: status success (with proof-uri)" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: work submitted" "WorkSubmitted" "$(echo "$out" | jq -r .result.status)"
  assert_eq "milestone-status: proof_uri recorded" "ipfs://QmIntegrationTest" "$(echo "$out" | jq -r .result.proof_uri)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json approve-release --agreement-id "$id" --milestone-id 0 --yes 2>&1)
  assert_eq "approve-release: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: completed after approval" "Completed" "$(echo "$out" | jq -r .result.status)"
}

test_dispute_lifecycle() {
  local id
  id=$(agreement_id 2)

  TRELLIS_SOURCE_KEY=payer "$CLI_BIN" init \
    --agreement-id "$id" --payer "$PAYER" --payee "$PAYEE" \
    --token "$TOKEN" --resolver "$RESOLVER" --milestones "500" --yes >/dev/null 2>&1
  TRELLIS_SOURCE_KEY=payer "$CLI_BIN" lock-funds --agreement-id "$id" --milestone-id 0 --yes >/dev/null 2>&1

  local out
  out=$(TRELLIS_SOURCE_KEY=payee "$CLI_BIN" --json raise-dispute --agreement-id "$id" --milestone-id 0 --caller "$PAYEE" --yes 2>&1)
  assert_eq "raise-dispute: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: disputed" "Disputed" "$(echo "$out" | jq -r .result.status)"

  out=$(TRELLIS_SOURCE_KEY=resolver "$CLI_BIN" --json resolve-dispute --agreement-id "$id" --milestone-id 0 --refund-to-payer --yes 2>&1)
  assert_eq "resolve-dispute: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: refunded" "Refunded" "$(echo "$out" | jq -r .result.status)"
}

test_cancel_unfunded_milestone() {
  local id
  id=$(agreement_id 3)

  TRELLIS_SOURCE_KEY=payer "$CLI_BIN" init \
    --agreement-id "$id" --payer "$PAYER" --payee "$PAYEE" \
    --token "$TOKEN" --resolver "$RESOLVER" --milestones "750" --yes >/dev/null 2>&1

  local out
  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json cancel-milestone --agreement-id "$id" --milestone-id 0 --yes 2>&1)
  assert_eq "cancel-milestone: status success" "success" "$(echo "$out" | jq -r .status)"

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json milestone-status --agreement-id "$id" --milestone-id 0 2>&1)
  assert_eq "milestone-status: cancelled after cancel" "Cancelled" "$(echo "$out" | jq -r .result.status)"
}

test_error_paths() {
  local out status_code

  # Malformed agreement ID must be rejected client-side, before ever
  # reaching the network (no tx_hash / RPC round trip).
  set +e
  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" status --agreement-id "deadbeef" 2>&1)
  status_code=$?
  set -e
  assert_eq "status: malformed agreement_id exits non-zero" "1" "$status_code"
  assert_contains "status: malformed agreement_id error message" "$out" "64 hex characters"

  # Double-init on an existing agreement must fail (contract-level error,
  # round-trips to the network — confirms error passthrough on failure).
  local id
  id=$(agreement_id 4)
  TRELLIS_SOURCE_KEY=payer "$CLI_BIN" init \
    --agreement-id "$id" --payer "$PAYER" --payee "$PAYEE" \
    --token "$TOKEN" --resolver "$RESOLVER" --milestones "100" --yes >/dev/null 2>&1

  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --json init \
    --agreement-id "$id" --payer "$PAYER" --payee "$PAYEE" \
    --token "$TOKEN" --resolver "$RESOLVER" --milestones "100" --yes 2>&1) || true
  assert_eq "init: double-init reports error status" "error" "$(echo "$out" | jq -r .status)"

  # A milestone action from an unauthorized identity must fail.
  out=$(TRELLIS_SOURCE_KEY=payee "$CLI_BIN" --json lock-funds --agreement-id "$id" --milestone-id 0 --yes 2>&1) || true
  assert_eq "lock-funds: wrong signer reports error status" "error" "$(echo "$out" | jq -r .status)"

  # --dry-run never touches the network and never prompts.
  out=$(TRELLIS_SOURCE_KEY=payer "$CLI_BIN" --dry-run approve-release --agreement-id "$id" --milestone-id 0 2>&1)
  assert_contains "approve-release --dry-run prints the invoke command" "$out" "stellar contract invoke"
}

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

log "running CLI integration tests against contract $CONTRACT_ID"
test_full_lifecycle
test_dispute_lifecycle
test_cancel_unfunded_milestone
test_error_paths

echo
log "$PASS_COUNT passed, $FAIL_COUNT failed"
if [ "$FAIL_COUNT" -gt 0 ]; then
  echo "Failures:"
  printf '  - %s\n' "${FAILURES[@]}"
  exit 1
fi
