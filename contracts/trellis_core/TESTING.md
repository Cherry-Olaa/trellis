# Testing Guide

This document describes the comprehensive test suite for the Trellis escrow contract.

## Test Categories

### 1. Example-Based Tests (`src/test.rs`)

Traditional unit tests covering specific scenarios:

```bash
cargo test
```

**Coverage includes:**
- Happy path (init → lock → submit → approve → release)
- Error paths (double init, invalid state transitions)
- Dispute flows (raise → resolve)
- Multi-milestone agreements
- Batch operations

### 2. Property-Based Tests (`src/test_properties.rs`)

Randomized tests that verify invariants across thousands of input combinations using `proptest`:

```bash
# Run with default case count (256)
cargo test test_properties

# Run with expanded case count for thorough coverage
PROPTEST_CASES=10000 cargo test test_properties
```

**Invariants verified:**
1. **Balance conservation** — contract balance always equals locked funds
2. **State machine integrity** — transitions follow valid paths
3. **Zero/negative amount rejection** — invalid milestones always fail
4. **Total amount integrity** — sum matches pre-computed total
5. **Milestone state isolation** — operations on one milestone don't affect others

### 3. Gas Cost Benchmarks (`benches/contract_bench.rs`)

Measures CPU and memory costs for all contract entrypoints:

```bash
cargo bench --bench contract_bench
```

**Output format:**
```json
[
  {"benchmark":"init_1_milestone","cpu_instructions":12345,"memory_bytes":6789},
  {"benchmark":"lock_funds","cpu_instructions":23456,"memory_bytes":7890},
  ...
]
```

Use this JSON output in CI to detect gas regressions (>10% increase).

**Benchmarked operations:**
- `init` (1 milestone and 10 milestones)
- `lock_funds` (1 milestone and 10 milestones — the 10-milestone case is the
  regression guard for #401: a milestone state transition must cost the same
  regardless of how many milestones the agreement has)
- `submit_work`
- `approve_and_release`
- `raise_dispute`
- `resolve_dispute`
- `batch_lock_funds` (5 milestones)
- `get_agreement`

### 4. Mutation Testing (`mutants.toml`)

Verifies test suite effectiveness by introducing code mutations and checking if tests catch them:

```bash
# Install cargo-mutants
cargo install --locked cargo-mutants

# Run mutation testing (targets >80% kill rate)
cargo mutants

# Run on specific files
cargo mutants --file src/lib.rs

# Run with timeout adjustment
cargo mutants --timeout 120
```

**Configuration:**
- Focused on `src/lib.rs`, `src/errors.rs`, `src/types.rs`
- Excludes infrastructure code (`events.rs`, `storage.rs`)
- 60-second timeout per mutant
- Release mode for faster feedback

**Interpreting results:**
- **Caught mutants** — tests correctly detect the mutation (good)
- **Missed mutants** — mutation not detected, indicates weak test coverage
- **Timeout mutants** — mutation caused tests to hang
- **Build failed** — mutation broke compilation

**Target:** >80% mutation kill rate

### 5. Snapshot Tests (`test_snapshots/`)

Property tests generate deterministic snapshots of contract events and state:

```bash
# Review snapshot changes
cargo insta review

# Accept all pending snapshots
cargo insta accept
```

## CI Integration

### GitHub Actions Example

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    
    # Example-based and property tests
    - name: Run tests
      run: cargo test
    
    # Property tests with expanded case count
    - name: Run property tests (extended)
      run: PROPTEST_CASES=8192 cargo test test_properties
    
    # Gas benchmarks (save for comparison)
    - name: Run gas benchmarks
      run: cargo bench --bench contract_bench > gas_costs.json
    
    # Mutation testing (weekly schedule)
    - name: Run mutation tests
      if: github.event.schedule == '0 0 * * 0'  # Sundays only
      run: |
        cargo install --locked cargo-mutants
        cargo mutants --output mutants.json
    
    # Gas regression check
    - name: Check gas costs
      run: |
        python scripts/compare_gas_costs.py baseline.json gas_costs.json
```

## Test Development Guidelines

### When to Add Example-Based Tests

- New entrypoint or command
- Specific bug fix (regression test)
- Complex business logic path
- Integration of multiple components

### When to Add Property-Based Tests

- Invariants that should hold for all inputs
- State machine correctness across random sequences
- Balance/amount calculations
- Cross-milestone interactions

### When to Update Benchmarks

- New entrypoint added
- Significant algorithm change
- After optimization work
- Before/after SDK upgrades

## Performance Targets

| Operation | CPU (max) | Memory (max) |
|-----------|-----------|--------------|
| init (1 milestone) | TBD | TBD |
| lock_funds | TBD | TBD |
| approve_and_release | TBD | TBD |

Run benchmarks to establish baseline, then flag PRs that exceed thresholds.

## Debugging Failed Tests

### Property Test Failures

Proptest automatically shrinks failing inputs to minimal reproducible cases:

```bash
# Failed test shows:
# thread 'prop_balance_conservation_happy_path' panicked at ...
# minimal failing input: amounts = [1, 2]

# Run with specific seed to reproduce
PROPTEST_SEED=12345 cargo test prop_balance_conservation_happy_path
```

### Gas Regression Failures

```bash
# Run benchmark on both commits
git checkout main
cargo bench --bench contract_bench > baseline.json

git checkout feature-branch
cargo bench --bench contract_bench > current.json

# Compare
diff baseline.json current.json
```

### Mutation Test Failures (Missed Mutants)

Example: Mutant changes `if status == Pending` to `if status != Pending` and tests still pass.

**Fix:** Add test covering the Pending status specifically:

```rust
#[test]
fn test_cancel_only_works_on_pending() {
    // ... setup ...
    client.lock_funds(&id, &0);  // Now Funded, not Pending
    
    let result = client.try_cancel_unfunded_milestone(&id, &0);
    assert_eq!(result, Err(Ok(TrellisError::InvalidStateTransition)));
}
```

## Resources

- [proptest documentation](https://docs.rs/proptest/)
- [cargo-mutants guide](https://mutants.rs/)
- [Soroban testing guide](https://developers.stellar.org/docs/smart-contracts/testing)
