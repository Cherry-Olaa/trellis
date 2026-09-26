//! Panic-boundary tests for the Trellis contract (#154).
//!
//! In Soroban a panic inside contract code traps the host: the whole
//! transaction reverts **but the caller is still charged the fee**, and an
//! unexpected trap mid-execution can leave callers reasoning about an
//! inconsistent state. Every entrypoint must therefore turn bad input —
//! unknown agreement IDs, out-of-range milestone indices, illegal state
//! transitions — into a typed [`TrellisError`] that is returned normally,
//! never into a panic/`unwrap`/`expect`.
//!
//! The contract source (`lib.rs`, `storage.rs`) contains **no** `unwrap()` or
//! `expect()` calls: milestone lookups go through
//! `Vec::get(..).ok_or(TrellisError::InvalidMilestone)?` and storage reads
//! through `Option::ok_or(TrellisError::AgreementNotFound)?`. These tests lock
//! that property in place by driving each entrypoint down its failure paths
//! via the generated `try_*` client methods and asserting the result is a
//! graceful `Err(Ok(TrellisError::_))` (contract-level error) rather than
//! `Err(Err(_))` (a host trap) or an outright test panic.
//!
//! A custom `#[panic_handler]` is intentionally *not* added: a `soroban-sdk`
//! contract compiled to wasm already gets its panic handler from the SDK, and
//! defining a second one is a duplicate-lang-item error. The SDK's handler is
//! the panic boundary; the job here is to make sure no contract codepath
//! reaches it.

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, Vec};

use crate::{
    errors::TrellisError,
    types::{EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

// ---------------------------------------------------------------------------
// Helpers (kept local, mirroring test.rs / test_properties.rs)
// ---------------------------------------------------------------------------

fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn setup() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    let env = Env::default();
    // Auth is mocked so these tests isolate *input handling*: we want to reach
    // the index / state / id checks, not stop at a signature check.
    env.mock_all_auths();

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let dispute_resolver = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_address).mint(&payer, &1_000_000_000);

    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

/// Build a `Vec<Milestone>` of `n` Pending milestones worth 1_000 each.
fn pending_milestones(env: &Env, n: u32) -> Vec<Milestone> {
    let mut v: Vec<Milestone> = Vec::new(env);
    for _ in 0..n {
        v.push_back(Milestone {
            amount: 1_000,
            status: EscrowStatus::Pending,
            proof_uri: None,
        });
    }
    v
}

/// Build a `Vec<Milestone>` from explicit amounts (used for the `init` edge
/// cases that need zero / negative values).
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

#[allow(clippy::too_many_arguments)]
fn init_agreement(
    client: &TrellisContractClient<'static>,
    env: &Env,
    seed: u8,
    payer: &Address,
    payee: &Address,
    token: &Address,
    resolver: &Address,
    n: u32,
) -> BytesN<32> {
    let id = agreement_id(env, seed);
    client.init(
        &id,
        payer,
        payee,
        token,
        &pending_milestones(env, n),
        resolver,
    );
    id
}

// ---------------------------------------------------------------------------
// Explicit failure-path cases — one panic-prone path per entrypoint
// ---------------------------------------------------------------------------

#[test]
fn unknown_agreement_id_never_panics() {
    let (env, payer, payee, _resolver, _token, client) = setup();
    let missing = agreement_id(&env, 200);

    // Every entrypoint that reads an agreement must surface AgreementNotFound.
    assert_eq!(
        client.try_lock_funds(&missing, &0),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_submit_work(&missing, &0, &None),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_approve_and_release(&missing, &0),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_raise_dispute(&payee, &missing, &0),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_resolve_dispute(&missing, &0, &true),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_cancel_unfunded_milestone(&missing, &0),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    assert_eq!(
        client.try_extend_agreement_ttl(&missing, &payer),
        Err(Ok(TrellisError::AgreementNotFound))
    );
    // Non-`()` success types can't derive PartialEq, so match instead of eq.
    assert!(matches!(
        client.try_get_agreement(&missing),
        Err(Ok(TrellisError::AgreementNotFound))
    ));
    // Option-returning view: absence, not a trap.
    assert!(matches!(
        client.try_get_milestone(&missing, &0),
        Ok(Ok(None))
    ));
}

#[test]
fn out_of_range_milestone_index_never_panics() {
    let (env, payer, payee, resolver, token, client) = setup();
    let id = init_agreement(&client, &env, 1, &payer, &payee, &token, &resolver, 1);

    // Index 999 is far past the single milestone — the `Vec::get(..).ok_or(..)`
    // in lib.rs must convert this to InvalidMilestone, not index-panic.
    let oob = 999u32;
    assert_eq!(
        client.try_lock_funds(&id, &oob),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_submit_work(&id, &oob, &None),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_approve_and_release(&id, &oob),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_raise_dispute(&payee, &id, &oob),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_resolve_dispute(&id, &oob, &false),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_cancel_unfunded_milestone(&id, &oob),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert!(matches!(client.try_get_milestone(&id, &oob), Ok(Ok(None))));
}

#[test]
fn u32_max_milestone_index_never_panics() {
    let (env, payer, payee, resolver, token, client) = setup();
    let id = init_agreement(&client, &env, 2, &payer, &payee, &token, &resolver, 3);

    assert_eq!(
        client.try_lock_funds(&id, &u32::MAX),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_approve_and_release(&id, &u32::MAX),
        Err(Ok(TrellisError::InvalidMilestone))
    );
}

#[test]
fn illegal_state_transitions_never_panic() {
    let (env, payer, payee, resolver, token, client) = setup();
    let id = init_agreement(&client, &env, 3, &payer, &payee, &token, &resolver, 1);

    // Milestone 0 is Pending. Submitting / approving before funding, or
    // resolving a milestone that was never disputed, must be a typed error.
    assert_eq!(
        client.try_submit_work(&id, &0, &None),
        Err(Ok(TrellisError::InvalidStateTransition))
    );
    assert_eq!(
        client.try_approve_and_release(&id, &0),
        Err(Ok(TrellisError::InvalidStateTransition))
    );
    assert_eq!(
        client.try_resolve_dispute(&id, &0, &true),
        Err(Ok(TrellisError::InvalidStateTransition))
    );
    assert_eq!(
        client.try_raise_dispute(&payee, &id, &0),
        Err(Ok(TrellisError::InvalidStateTransition))
    );

    // Fund it, then try to cancel — cancel only applies to Pending milestones.
    client.lock_funds(&id, &0);
    assert_eq!(
        client.try_cancel_unfunded_milestone(&id, &0),
        Err(Ok(TrellisError::InvalidStateTransition))
    );
}

#[test]
fn raise_dispute_with_non_party_caller_never_panics() {
    let (env, payer, payee, resolver, token, client) = setup();
    let id = init_agreement(&client, &env, 4, &payer, &payee, &token, &resolver, 1);
    client.lock_funds(&id, &0);

    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_raise_dispute(&stranger, &id, &0),
        Err(Ok(TrellisError::Unauthorized))
    );
}

#[test]
fn init_edge_cases_return_typed_errors() {
    let (env, payer, payee, resolver, token, client) = setup();

    // Empty milestone set.
    let empty: Vec<Milestone> = Vec::new(&env);
    assert_eq!(
        client.try_init(
            &agreement_id(&env, 10),
            &payer,
            &payee,
            &token,
            &empty,
            &resolver
        ),
        Err(Ok(TrellisError::EmptyMilestoneSet))
    );

    // Resolver is also the payer.
    assert_eq!(
        client.try_init(
            &agreement_id(&env, 11),
            &payer,
            &payee,
            &token,
            &milestones_from_amounts(&env, &[1_000]),
            &payer,
        ),
        Err(Ok(TrellisError::ResolverCannotBeParty))
    );

    // Zero / negative milestone amount.
    assert_eq!(
        client.try_init(
            &agreement_id(&env, 12),
            &payer,
            &payee,
            &token,
            &milestones_from_amounts(&env, &[0]),
            &resolver,
        ),
        Err(Ok(TrellisError::InvalidMilestone))
    );
    assert_eq!(
        client.try_init(
            &agreement_id(&env, 13),
            &payer,
            &payee,
            &token,
            &milestones_from_amounts(&env, &[-1]),
            &resolver,
        ),
        Err(Ok(TrellisError::InvalidMilestone))
    );
}

// ---------------------------------------------------------------------------
// Fuzz: random milestone indices / agreement seeds never trap the host
// ---------------------------------------------------------------------------

/// `Err(Err(_))` from a `try_*` call is a host trap (a contract panic). Any
/// other shape — `Ok(_)` or `Err(Ok(_))` — means the contract handled the
/// input gracefully.
macro_rules! assert_no_trap {
    ($result:expr, $label:expr) => {{
        let r = $result;
        prop_assert!(
            !matches!(r, Err(Err(_))),
            "{} trapped the host instead of returning a typed error: {:?}",
            $label,
            r
        );
    }};
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For an agreement with 1..=4 milestones, calling every milestone-indexed
    /// entrypoint with an arbitrary `u32` index must never trap — regardless of
    /// whether the index is in range, just past the end, or `u32::MAX`.
    #[test]
    fn fuzz_arbitrary_milestone_index_never_traps(
        n_milestones in 1u32..=4,
        milestone_id in any::<u32>(),
        seed in any::<u8>(),
    ) {
        let (env, payer, payee, resolver, token, client) = setup();
        let id = init_agreement(&client, &env, seed, &payer, &payee, &token, &resolver, n_milestones);

        assert_no_trap!(client.try_lock_funds(&id, &milestone_id), "lock_funds");
        assert_no_trap!(client.try_submit_work(&id, &milestone_id, &None), "submit_work");
        assert_no_trap!(client.try_approve_and_release(&id, &milestone_id), "approve_and_release");
        assert_no_trap!(client.try_raise_dispute(&payee, &id, &milestone_id), "raise_dispute");
        assert_no_trap!(client.try_resolve_dispute(&id, &milestone_id, &true), "resolve_dispute");
        assert_no_trap!(client.try_cancel_unfunded_milestone(&id, &milestone_id), "cancel");
        assert_no_trap!(client.try_get_milestone(&id, &milestone_id), "get_milestone");
    }

    /// Calling entrypoints against a randomly seeded (and almost certainly
    /// non-existent) agreement ID must never trap.
    #[test]
    fn fuzz_arbitrary_agreement_id_never_traps(
        seed in any::<u8>(),
        milestone_id in any::<u32>(),
    ) {
        let (env, payer, payee, _resolver, _token, client) = setup();
        let id = agreement_id(&env, seed);

        assert_no_trap!(client.try_lock_funds(&id, &milestone_id), "lock_funds");
        assert_no_trap!(client.try_submit_work(&id, &milestone_id, &None), "submit_work");
        assert_no_trap!(client.try_approve_and_release(&id, &milestone_id), "approve_and_release");
        assert_no_trap!(client.try_raise_dispute(&payee, &id, &milestone_id), "raise_dispute");
        assert_no_trap!(client.try_resolve_dispute(&id, &milestone_id, &false), "resolve_dispute");
        assert_no_trap!(client.try_cancel_unfunded_milestone(&id, &milestone_id), "cancel");
        assert_no_trap!(client.try_get_agreement(&id), "get_agreement");
        assert_no_trap!(client.try_get_milestone(&id, &milestone_id), "get_milestone");
        assert_no_trap!(client.try_extend_agreement_ttl(&id, &payer), "extend_agreement_ttl");
    }

    /// A random proof URI of arbitrary length / content must not trap
    /// `submit_work` — it fails earlier on state, but the string handling
    /// itself must be panic-free.
    #[test]
    fn fuzz_arbitrary_proof_uri_never_traps(proof in ".*") {
        let (env, payer, payee, resolver, token, client) = setup();
        let id = init_agreement(&client, &env, 77, &payer, &payee, &token, &resolver, 1);
        let uri = Some(soroban_sdk::String::from_str(&env, &proof));
        assert_no_trap!(client.try_submit_work(&id, &0, &uri), "submit_work");
    }
}
