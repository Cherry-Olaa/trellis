//! Property-based invariant tests for the Trellis escrow state machine.
//!
//! These tests complement the example-based tests in `test.rs` by exercising
//! the contract with randomly generated inputs and verifying that core
//! invariants hold for all of them. Run with an expanded case count via:
//!
//! ```bash
//! PROPTEST_CASES=10000 cargo test test_properties
//! ```
//!
//! # Invariants under test
//!
//! 1. **Balance conservation** — the token balance held by the contract always
//!    equals the sum of amounts for milestones that are in `Funded`,
//!    `WorkSubmitted`, or `Disputed` state. Funds never vanish or appear from
//!    nowhere.
//!
//! 2. **State machine reachability** — `approve_and_release` always fails with
//!    `InvalidStateTransition` unless the milestone is `WorkSubmitted`.
//!
//! 3. **Zero/negative-amount rejection** — `init` always fails with
//!    `InvalidMilestone` when any milestone has `amount <= 0`, regardless of
//!    how many milestones are in the list.
//!
//! 4. **total_amount integrity** — the `total_amount` field on an agreement
//!    always equals the sum of its milestones' individual amounts.
//!
//! 5. **Independent milestone states** — completing or cancelling one milestone
//!    in a multi-milestone agreement never changes the state of any other
//!    milestone.
//!
//! 6. **Balance conservation under random operation ordering** (#141) — same
//!    invariant as #1, but instead of one fixed lock→submit→approve pass per
//!    milestone, a random *sequence* of operations (lock/submit/approve/
//!    dispute/resolve/cancel) is generated and applied across all of an
//!    agreement's milestones in random order, skipping whichever operations
//!    aren't a legal transition from that milestone's current state. This
//!    exercises interleavings the other invariants never construct — e.g.
//!    disputing milestone 2 while milestone 0 is mid-approval — which is
//!    where race-condition-shaped bugs between milestones would show up.
//!    Sequences run up to 60 ops long; raise the case count for a deeper
//!    sweep with `PROPTEST_CASES=10000 cargo test test_properties`.

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, Vec};

use crate::{
    errors::TrellisError,
    types::{EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

// ---------------------------------------------------------------------------
// Helpers shared with the property tests
// ---------------------------------------------------------------------------

fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Spin up a fresh Soroban environment with a funded payer.
///
/// Returns `(env, payer, payee, dispute_resolver, token_address, client)`.
fn setup() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let dispute_resolver = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    // Mint generously so any combination of milestone amounts can be funded.
    token_admin_client.mint(&payer, &1_000_000_000);

    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

/// Build a `Vec<Milestone>` from a slice of amounts. All statuses are Pending.
fn milestones_from_amounts(env: &Env, amounts: &[i128]) -> Vec<Milestone> {
    let mut v: Vec<Milestone> = Vec::new(env);
    for &amount in amounts.iter() {
        v.push_back(Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
        });
    }
    v
}

// ---------------------------------------------------------------------------
// Invariant 1 — Balance conservation
// ---------------------------------------------------------------------------

proptest! {
    /// Verify balance conservation through a complete happy-path sequence for a
    /// randomly sized multi-milestone agreement (1–5 milestones, each 1–10_000).
    /// After every lock/release pair the contract balance must track exactly.
    #[test]
    fn prop_balance_conservation_happy_path(
        amounts in prop::collection::vec(1i128..=10_000i128, 1..=5usize),
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let token_client = token::TokenClient::new(&env, &token_address);
        let id = agreement_id(&env, 42);

        let milestones = milestones_from_amounts(&env, &amounts);
        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        let mut expected_locked: i128 = 0;

        for (i, &amount) in amounts.iter().enumerate() {
            let mid = i as u32;

            // Before locking: contract holds `expected_locked`.
            prop_assert_eq!(
                token_client.balance(&client.address),
                expected_locked,
                "contract balance before lock of milestone {} should be {}", i, expected_locked
            );

            client.lock_funds(&id, &mid);
            expected_locked += amount;

            prop_assert_eq!(
                token_client.balance(&client.address),
                expected_locked,
                "contract balance after lock of milestone {} should be {}", i, expected_locked
            );

            // Submit does not change token balances.
            client.submit_work(&id, &mid, &None);

            prop_assert_eq!(
                token_client.balance(&client.address),
                expected_locked,
                "contract balance must not change on submit of milestone {}", i
            );

            client.approve_and_release(&id, &mid);
            expected_locked -= amount;

            prop_assert_eq!(
                token_client.balance(&client.address),
                expected_locked,
                "contract balance after release of milestone {} should be {}", i, expected_locked
            );
        }

        // All milestones released — contract must hold nothing.
        prop_assert_eq!(
            token_client.balance(&client.address),
            0,
            "contract must hold zero after all milestones released"
        );
    }

    /// Verify balance conservation through a dispute → refund-to-payer sequence.
    #[test]
    fn prop_balance_conservation_dispute_refund(
        amounts in prop::collection::vec(1i128..=10_000i128, 1..=3usize),
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let token_client = token::TokenClient::new(&env, &token_address);
        let id = agreement_id(&env, 43);

        let milestones = milestones_from_amounts(&env, &amounts);
        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        let mut locked: i128 = 0;

        for (i, &amount) in amounts.iter().enumerate() {
            let mid = i as u32;

            client.lock_funds(&id, &mid);
            locked += amount;

            // Balance must not change when raising a dispute.
            client.raise_dispute(&payer, &id, &mid);

            prop_assert_eq!(
                token_client.balance(&client.address),
                locked,
                "contract balance must be unchanged after dispute raised for milestone {}", i
            );

            // Resolve in payer's favour (refund).
            client.resolve_dispute(&id, &mid, &true);
            locked -= amount;

            prop_assert_eq!(
                token_client.balance(&client.address),
                locked,
                "contract balance after refund for milestone {} should be {}", i, locked
            );
        }

        prop_assert_eq!(
            token_client.balance(&client.address),
            0,
            "contract must hold zero after all milestones refunded via dispute"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 2 — State machine: approve_and_release requires WorkSubmitted
// ---------------------------------------------------------------------------

proptest! {
    /// approve_and_release on a Pending milestone always fails, regardless of
    /// the milestone amount.
    #[test]
    fn prop_approve_on_pending_always_fails(amount in 1i128..=100_000i128) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 50);

        client.init(
            &id, &payer, &payee, &token_address,
            &milestones_from_amounts(&env, &[amount]),
            &dispute_resolver,
        );

        let result = client.try_approve_and_release(&id, &0u32);
        prop_assert_eq!(
            result,
            Err(Ok(TrellisError::InvalidStateTransition)),
            "approve on Pending must always fail"
        );
    }

    /// approve_and_release on a Funded milestone always fails, regardless of
    /// the milestone amount.
    #[test]
    fn prop_approve_on_funded_always_fails(amount in 1i128..=100_000i128) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 51);

        client.init(
            &id, &payer, &payee, &token_address,
            &milestones_from_amounts(&env, &[amount]),
            &dispute_resolver,
        );
        client.lock_funds(&id, &0u32);

        let result = client.try_approve_and_release(&id, &0u32);
        prop_assert_eq!(
            result,
            Err(Ok(TrellisError::InvalidStateTransition)),
            "approve on Funded must always fail"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 3 — Zero/negative amount always rejected
// ---------------------------------------------------------------------------

proptest! {
    /// init with a single zero-amount milestone always returns InvalidMilestone.
    #[test]
    fn prop_zero_amount_always_rejected(seed in 0u8..=200u8) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        // Use a different ID each run to avoid AlreadyInitialized masking the error.
        let id = BytesN::from_array(&env, &[seed; 32]);

        let result = client.try_init(
            &id, &payer, &payee, &token_address,
            &milestones_from_amounts(&env, &[0]),
            &dispute_resolver,
        );
        prop_assert_eq!(
            result,
            Err(Ok(TrellisError::InvalidMilestone)),
            "zero-amount milestone must always be rejected"
        );
    }

    /// init with a negative milestone amount always returns InvalidMilestone.
    #[test]
    fn prop_negative_amount_always_rejected(amount in i128::MIN..=-1i128) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 60);

        let result = client.try_init(
            &id, &payer, &payee, &token_address,
            &milestones_from_amounts(&env, &[amount]),
            &dispute_resolver,
        );
        prop_assert_eq!(
            result,
            Err(Ok(TrellisError::InvalidMilestone)),
            "negative-amount milestone must always be rejected"
        );
    }

    /// init with a mix of valid amounts and one zero always returns
    /// InvalidMilestone, regardless of the zero's position.
    #[test]
    fn prop_mixed_zero_always_rejected(
        prefix_amounts in prop::collection::vec(1i128..=10_000i128, 0..=3usize),
        suffix_amounts in prop::collection::vec(1i128..=10_000i128, 0..=3usize),
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 61);

        // Interleave a zero in the middle.
        let mut all_amounts = prefix_amounts.clone();
        all_amounts.push(0);
        all_amounts.extend(suffix_amounts.iter().copied());

        let result = client.try_init(
            &id, &payer, &payee, &token_address,
            &milestones_from_amounts(&env, &all_amounts),
            &dispute_resolver,
        );
        prop_assert_eq!(
            result,
            Err(Ok(TrellisError::InvalidMilestone)),
            "mixed milestone list with a zero must always be rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 4 — total_amount == sum of milestone amounts
// ---------------------------------------------------------------------------

proptest! {
    /// The pre-computed total_amount on any successfully created agreement
    /// equals the arithmetic sum of all its milestone amounts.
    #[test]
    fn prop_total_amount_equals_sum(
        amounts in prop::collection::vec(1i128..=10_000i128, 1..=8usize),
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 70);

        let expected_total: i128 = amounts.iter().sum();
        let milestones = milestones_from_amounts(&env, &amounts);

        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        let agreement = client.get_agreement(&id);
        prop_assert_eq!(
            agreement.total_amount,
            expected_total,
            "total_amount must equal the arithmetic sum of milestone amounts"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 5 — milestone state isolation in multi-milestone agreements
// ---------------------------------------------------------------------------

proptest! {
    /// Completing milestone 0 in a two-milestone agreement must not change
    /// the state of milestone 1, regardless of milestone amounts.
    #[test]
    fn prop_milestone_completion_does_not_affect_neighbours(
        amount0 in 1i128..=10_000i128,
        amount1 in 1i128..=10_000i128,
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 80);

        let milestones = milestones_from_amounts(&env, &[amount0, amount1]);
        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        // Complete milestone 0 through the full happy path.
        client.lock_funds(&id, &0u32);
        client.submit_work(&id, &0u32, &None);
        client.approve_and_release(&id, &0u32);

        // Milestone 1 must still be Pending.
        let agreement = client.get_agreement(&id);
        let m1 = agreement.milestones.get(1).expect("milestone 1 must exist");
        prop_assert_eq!(
            m1.status,
            EscrowStatus::Pending,
            "milestone 1 must remain Pending after milestone 0 is completed"
        );
    }

    /// Cancelling milestone 0 must not affect milestone 1, regardless of amounts.
    #[test]
    fn prop_milestone_cancellation_does_not_affect_neighbours(
        amount0 in 1i128..=10_000i128,
        amount1 in 1i128..=10_000i128,
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let id = agreement_id(&env, 81);

        let milestones = milestones_from_amounts(&env, &[amount0, amount1]);
        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        // Cancel milestone 0 (still Pending, never funded).
        client.cancel_unfunded_milestone(&id, &0u32);

        // Milestone 1 must still be Pending.
        let agreement = client.get_agreement(&id);
        let m1 = agreement.milestones.get(1).expect("milestone 1 must exist");
        prop_assert_eq!(
            m1.status,
            EscrowStatus::Pending,
            "milestone 1 must remain Pending after milestone 0 is cancelled"
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 6 — balance conservation under randomly ordered operation sequences
// ---------------------------------------------------------------------------

/// One step of a randomly generated operation sequence. Which milestone it
/// targets is chosen separately (see `prop_balance_conservation_random_op_sequence`).
#[derive(Debug, Clone, Copy)]
enum FuzzOp {
    Lock,
    Submit,
    Approve,
    Dispute,
    ResolveRefund,
    ResolveRelease,
    Cancel,
}

fn fuzz_op_strategy() -> impl Strategy<Value = FuzzOp> {
    prop_oneof![
        Just(FuzzOp::Lock),
        Just(FuzzOp::Submit),
        Just(FuzzOp::Approve),
        Just(FuzzOp::Dispute),
        Just(FuzzOp::ResolveRefund),
        Just(FuzzOp::ResolveRelease),
        Just(FuzzOp::Cancel),
    ]
}

proptest! {
    /// Applies a random sequence of operations, targeting randomly chosen
    /// milestones in a multi-milestone agreement, in random order — skipping
    /// any operation that isn't a legal transition from that milestone's
    /// current (locally tracked) state. After every operation that *is*
    /// applied, the contract's token balance must equal the sum of amounts
    /// this test believes are still locked (milestones currently Funded,
    /// WorkSubmitted, or Disputed) — verifying balance conservation holds
    /// under adversarial interleaving across milestones, not just the one
    /// fixed per-milestone lock→submit→approve pass every other test uses.
    #[test]
    fn prop_balance_conservation_random_op_sequence(
        amounts in prop::collection::vec(1i128..=10_000i128, 2..=4usize),
        ops in prop::collection::vec((0usize..4, fuzz_op_strategy()), 20..=60usize),
    ) {
        let (env, payer, payee, dispute_resolver, token_address, client) = setup();
        let token_client = token::TokenClient::new(&env, &token_address);
        let id = agreement_id(&env, 90);

        let milestone_count = amounts.len();
        let milestones = milestones_from_amounts(&env, &amounts);
        client.init(&id, &payer, &payee, &token_address, &milestones, &dispute_resolver);

        // Local shadow model of each milestone's status, mirroring the
        // contract's authorized state machine — not read back from the
        // contract, so a divergence between this model and on-chain state
        // shows up as a wrong `expected_balance` rather than being masked.
        // `amounts` is generated with 2..=4 entries; a fixed-size array
        // avoids needing an allocator in this `#![no_std]` crate.
        let mut status = [
            EscrowStatus::Pending,
            EscrowStatus::Pending,
            EscrowStatus::Pending,
            EscrowStatus::Pending,
        ];
        let mut expected_balance: i128 = 0;

        for (raw_index, op) in ops {
            let i = raw_index % milestone_count;
            let mid = i as u32;

            let applies = matches!(
                (status[i].clone(), op),
                (EscrowStatus::Pending, FuzzOp::Lock)
                    | (EscrowStatus::Pending, FuzzOp::Cancel)
                    | (EscrowStatus::Funded, FuzzOp::Submit)
                    | (EscrowStatus::Funded, FuzzOp::Dispute)
                    | (EscrowStatus::WorkSubmitted, FuzzOp::Approve)
                    | (EscrowStatus::WorkSubmitted, FuzzOp::Dispute)
                    | (EscrowStatus::Disputed, FuzzOp::ResolveRefund)
                    | (EscrowStatus::Disputed, FuzzOp::ResolveRelease)
            );

            if !applies {
                continue;
            }

            match op {
                FuzzOp::Lock => {
                    client.lock_funds(&id, &mid);
                    status[i] = EscrowStatus::Funded;
                    expected_balance += amounts[i];
                }
                FuzzOp::Submit => {
                    client.submit_work(&id, &mid, &None);
                    status[i] = EscrowStatus::WorkSubmitted;
                }
                FuzzOp::Approve => {
                    client.approve_and_release(&id, &mid);
                    status[i] = EscrowStatus::Completed;
                    expected_balance -= amounts[i];
                }
                FuzzOp::Dispute => {
                    client.raise_dispute(&payer, &id, &mid);
                    status[i] = EscrowStatus::Disputed;
                }
                FuzzOp::ResolveRefund => {
                    client.resolve_dispute(&id, &mid, &true);
                    status[i] = EscrowStatus::Refunded;
                    expected_balance -= amounts[i];
                }
                FuzzOp::ResolveRelease => {
                    client.resolve_dispute(&id, &mid, &false);
                    status[i] = EscrowStatus::Completed;
                    expected_balance -= amounts[i];
                }
                FuzzOp::Cancel => {
                    client.cancel_unfunded_milestone(&id, &mid);
                    status[i] = EscrowStatus::Refunded;
                }
            }

            prop_assert_eq!(
                token_client.balance(&client.address),
                expected_balance,
                "balance mismatch after applying {:?} to milestone {} (local status now {:?})",
                op, i, status[i]
            );
        }
    }
}
