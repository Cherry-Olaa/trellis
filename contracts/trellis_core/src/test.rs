use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    token, vec, Address, BytesN, Env, String, Symbol, TryFromVal, Vec,
};

use crate::{
    errors::TrellisError,
    types::{EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a 32-byte agreement ID from a seed byte.
fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Create a single Milestone at index 0 with the given amount.
fn one_milestone(env: &Env, amount: i128) -> Vec<Milestone> {
    vec![
        env,
        Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ]
}

/// Allow every `require_auth` call in the test environment to succeed.
///
/// The Trellis entrypoints gate on `agreement.<role>.require_auth()`, so a
/// blanket mock is what lets a test drive the happy path. Tests that assert an
/// authorisation failure call [`deny_all_auth`] first, which turns mocking off
/// and makes any `require_auth` trap instead.
fn allow_all_auth(env: &Env) {
    env.mock_all_auths();
}

/// Turn auth mocking off, so any `require_auth` in the contract traps.
///
/// This is the positive control for the authorisation tests: it proves the
/// failure they observe comes from the contract's own auth gate and not from
/// some unrelated setup mistake.
fn deny_all_auth(env: &Env) {
    env.set_auths(&[]);
}

/// Assert the Trellis contract's own events, in order, for the most recent
/// top-level invocation.
///
/// `env.events().all()` also carries the SAC's own mint/transfer events, and in
/// soroban-sdk 22 it only reports the events of the *latest* invocation — so
/// each entrypoint's events have to be checked straight after the call rather
/// than accumulated across the whole test. Events from other contracts (the
/// token) are filtered out by contract address.
fn assert_trellis_topics(env: &Env, contract: &Address, expected: &[Symbol], msg: &str) {
    let all_events = env.events().all();
    let mut matched = 0usize;
    for i in 0..all_events.len() {
        let (contract_id, topics, _data) = all_events.get_unchecked(i);
        if contract_id != *contract {
            continue;
        }
        let topic0 = Symbol::try_from_val(env, &topics.get_unchecked(0))
            .expect("event topic 0 must decode as a Symbol");
        assert!(
            matched < expected.len(),
            "more Trellis contract events fired than expected in the last invocation"
        );
        assert_eq!(topic0, expected[matched], "event {matched} name mismatch");
        matched += 1;
    }
    assert_eq!(
        matched,
        expected.len(),
        "{msg} (saw {} total events)",
        all_events.len()
    );
}

/// Common test fixture.
///
/// Returns `(env, payer, payee, dispute_resolver, token_address, client)`.
/// Auth is mocked for the whole environment — see [`allow_all_auth`].
fn setup() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    let env = Env::default();

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let dispute_resolver = Address::generate(&env);

    // Deploy the built-in Stellar Asset Contract and mint payer a balance.
    // The mint is authorised by the asset admin, so auth has to be mocked
    // before it — `env.mock_all_auths()` also covers the Trellis entrypoints.
    allow_all_auth(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    token_admin_client.mint(&payer, &10_000);

    // Register the Trellis contract.
    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full happy-path: init → lock → submit → release.
/// Verifies balances at each step and checks all 4 events were emitted.
#[test]
fn test_happy_path() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let token_client = token::TokenClient::new(&env, &token_address);
    let id = agreement_id(&env, 1);
    let amount: i128 = 1_000;

    // ── init ───────────────────────────────────────────────────────────────
    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, amount),
        &dispute_resolver,
    );
    assert_trellis_topics(
        &env,
        &client.address,
        &[symbol_short!("trls_crte")],
        "init must emit exactly one agreement_created event",
    );

    // ── lock_funds ─────────────────────────────────────────────────────────
    let payer_balance_before = token_client.balance(&payer);
    client.lock_funds(&id, &0u32);
    assert_trellis_topics(
        &env,
        &client.address,
        &[symbol_short!("trls_lckd")],
        "lock_funds must emit exactly one funds_locked event",
    );

    assert_eq!(
        token_client.balance(&payer),
        payer_balance_before - amount,
        "payer balance should decrease by milestone amount after lock"
    );
    assert_eq!(
        token_client.balance(&client.address),
        amount,
        "trellis contract balance should equal locked milestone amount"
    );

    // ── submit_work ────────────────────────────────────────────────────────
    let proof = Some(String::from_str(&env, "ipfs://test"));
    client.submit_work(&id, &0u32, &proof);
    assert_trellis_topics(
        &env,
        &client.address,
        &[symbol_short!("trls_sbmt")],
        "submit_work must emit exactly one work_submitted event",
    );

    // ── approve_and_release ────────────────────────────────────────────────
    client.approve_and_release(&id, &0u32);
    assert_trellis_topics(
        &env,
        &client.address,
        &[symbol_short!("trls_rlsd")],
        "approve_and_release must emit exactly one funds_released event",
    );

    assert_eq!(
        token_client.balance(&payee),
        amount,
        "payee should receive the milestone amount after release"
    );
    assert_eq!(
        token_client.balance(&client.address),
        0,
        "contract balance should be zero after release"
    );
}

/// Calling `init` twice with the same agreement_id must return AlreadyInitialized.
#[test]
fn test_double_init_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 2);

    // First init — must succeed.
    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    // Second init — must fail with AlreadyInitialized.
    let result = client.try_init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );
    assert_eq!(
        result,
        Err(Ok(TrellisError::AlreadyInitialized)),
        "second init with same ID must return AlreadyInitialized"
    );
}

/// Dispute raised by payee → dispute_resolver rules in payer's favour → payer refunded.
#[test]
fn test_dispute_and_refund_to_payer() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let token_client = token::TokenClient::new(&env, &token_address);
    let id = agreement_id(&env, 3);
    let amount: i128 = 2_000;

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, amount),
        &dispute_resolver,
    );

    let payer_balance_before_lock = token_client.balance(&payer);
    client.lock_funds(&id, &0u32);

    // Payee raises the dispute (exercises the either-party auth path).
    client.raise_dispute(&payee, &id, &0u32);

    // Resolver rules in payer's favour.
    client.resolve_dispute(&id, &0u32, &true);

    assert_eq!(
        token_client.balance(&payer),
        payer_balance_before_lock,
        "payer balance should be fully restored after refund"
    );
    assert_eq!(
        token_client.balance(&client.address),
        0,
        "contract balance should be zero after resolution"
    );
}

/// Cancel a milestone that was never funded, then verify a second cancel fails.
#[test]
fn test_cancel_unfunded_milestone() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 4);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 300),
        &dispute_resolver,
    );

    // First cancel — must succeed (milestone is still Pending).
    client.cancel_unfunded_milestone(&id, &0u32);

    // Second cancel — must fail (milestone is now Refunded, not Pending).
    let result = client.try_cancel_unfunded_milestone(&id, &0u32);
    assert_eq!(
        result,
        Err(Ok(TrellisError::InvalidStateTransition)),
        "second cancel on an already-Refunded milestone must return InvalidStateTransition"
    );
}

/// Cancelling a milestone that has already been funded must be rejected with
/// InvalidStateTransition — the milestone genuinely has funds locked, so the
/// error must reflect the state machine violation, not an economic one.
#[test]
fn test_cancel_funded_milestone_fails_with_invalid_state_transition() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 6);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 400),
        &dispute_resolver,
    );

    // Fund the milestone so it is no longer Pending.
    client.lock_funds(&id, &0u32);

    let result = client.try_cancel_unfunded_milestone(&id, &0u32);
    assert_eq!(
        result,
        Err(Ok(TrellisError::InvalidStateTransition)),
        "cancelling a Funded milestone must return InvalidStateTransition"
    );
}

/// Multi-milestone agreements should preserve independent state transitions.
#[test]
fn test_multi_milestone_transitions() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 6);

    let milestones = vec![
        &env,
        Milestone {
            amount: 1_000,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 2_000,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ];

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &milestones,
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    let proof = Some(String::from_str(&env, "ipfs://multi-milestone"));
    client.submit_work(&id, &0u32, &proof);

    client.approve_and_release(&id, &0u32);

    let agreement = client.get_agreement(&id);
    let first = agreement.milestones.get(0).expect("milestone 0 must exist");
    let second = agreement.milestones.get(1).expect("milestone 1 must exist");

    assert_eq!(first.status, EscrowStatus::Completed);
    assert_eq!(second.status, EscrowStatus::Pending);
}

/// batch_lock_funds funds every milestone in the supplied list in one call.
#[test]
fn test_batch_lock_funds() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let token_client = token::TokenClient::new(&env, &token_address);
    let id = agreement_id(&env, 10);

    let milestones = vec![
        &env,
        Milestone {
            amount: 500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ];

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &milestones,
        &dispute_resolver,
    );

    let milestone_ids = vec![&env, 0u32, 1u32];
    let funded = client.batch_lock_funds(&id, &milestone_ids);

    assert_eq!(funded, 2u32, "both milestones should be funded");
    assert_eq!(
        token_client.balance(&client.address),
        1_000,
        "contract balance should equal sum of locked milestones"
    );
    assert_eq!(
        token_client.balance(&payer),
        9_000,
        "payer balance should decrease by the total locked amount"
    );
}

/// batch_lock_funds short-circuits on the first already-funded milestone.
#[test]
fn test_batch_lock_funds_partial_failure() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 11);

    let milestones = vec![
        &env,
        Milestone {
            amount: 500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ];

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &milestones,
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    // milestone 0 is already Funded — the batch must fail atomically.
    let milestone_ids = vec![&env, 0u32, 1u32];
    let result = client.try_batch_lock_funds(&id, &milestone_ids);
    assert_eq!(
        result,
        Err(Ok(TrellisError::InvalidStateTransition)),
        "batch should fail when a milestone is not Pending"
    );
}

/// get_agreement returns the correct Agreement after init, and AgreementNotFound
/// for an ID that was never initialized.
#[test]
fn test_get_agreement() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 5);

    // Init with one milestone so there is something to read back.
    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 750),
        &dispute_resolver,
    );

    // ── Happy path: agreement exists ──────────────────────────────────────
    let agreement = client.get_agreement(&id);

    assert_eq!(agreement.payer, payer, "payer address must match");
    assert_eq!(agreement.payee, payee, "payee address must match");
    assert_eq!(
        agreement.milestones.len(),
        1,
        "should have exactly one milestone"
    );

    let milestone = agreement.milestones.get(0).expect("milestone 0 must exist");
    assert_eq!(
        milestone.status,
        crate::types::EscrowStatus::Pending,
        "freshly created milestone must be Pending"
    );
    assert_eq!(milestone.amount, 750, "milestone amount must match");

    // ── Not-found path: unknown ID returns AgreementNotFound ──────────────
    let fake_id = agreement_id(&env, 99); // never initialized
    let result = client.try_get_agreement(&fake_id);
    assert!(result.is_err(), "unknown agreement ID must return an error");
    assert_eq!(
        result.err().unwrap(),
        Ok(TrellisError::AgreementNotFound),
        "error must be AgreementNotFound"
    );
}

/// get_milestone returns the correct milestone for a valid index.
#[test]
fn test_get_milestone_returns_correct_milestone() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 20);

    let milestones = vec![
        &env,
        Milestone {
            amount: 100,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 200,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ];

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &milestones,
        &dispute_resolver,
    );

    let m = client.get_milestone(&id, &1u32);
    assert!(m.is_some(), "milestone 1 must be found");
    let m = m.unwrap();
    assert_eq!(m.amount, 200, "amount must match");
    assert_eq!(m.status, EscrowStatus::Pending, "status must be Pending");
}

/// get_milestone returns None for an out-of-range milestone_id.
#[test]
fn test_get_milestone_invalid_id_returns_none() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 21);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 100),
        &dispute_resolver,
    );

    let result = client.get_milestone(&id, &99u32);
    assert!(
        result.is_none(),
        "out-of-range milestone_id must return None"
    );
}

// ---------------------------------------------------------------------------
// Authorization tests
// ---------------------------------------------------------------------------

/// `lock_funds` is payer-only: without a payer signature the call traps.
#[test]
#[should_panic(expected = "InvalidAction")]
fn test_lock_funds_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 30);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    // `lock_funds` gates on `agreement.payer.require_auth()`. With auth
    // mocking off, a caller that has not signed the invocation traps.
    deny_all_auth(&env);
    client.lock_funds(&id, &0u32);
}

/// `submit_work` is payee-only: without a payee signature the call traps.
#[test]
#[should_panic(expected = "InvalidAction")]
fn test_submit_work_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 31);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    // `submit_work` gates on `agreement.payee.require_auth()`.
    deny_all_auth(&env);
    let proof = Some(String::from_str(&env, "ipfs://fake"));
    client.submit_work(&id, &0u32, &proof);
}

/// `approve_and_release` is payer-only: without a payer signature it traps.
#[test]
#[should_panic(expected = "InvalidAction")]
fn test_approve_release_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 32);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    let proof = Some(String::from_str(&env, "ipfs://work"));
    client.submit_work(&id, &0u32, &proof);

    // `approve_and_release` gates on `agreement.payer.require_auth()`.
    deny_all_auth(&env);
    client.approve_and_release(&id, &0u32);
}

/// `resolve_dispute` is resolver-only: without the resolver's signature it traps.
#[test]
#[should_panic(expected = "InvalidAction")]
fn test_resolve_dispute_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 33);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    client.raise_dispute(&payee, &id, &0u32);

    // `resolve_dispute` gates on `agreement.dispute_resolver.require_auth()`,
    // which is its sole role check — see the entrypoint's doc comment.
    deny_all_auth(&env);
    client.resolve_dispute(&id, &0u32, &true);
}

/// `raise_dispute` is party-only: a caller that is neither payer nor payee is
/// rejected with a typed `Unauthorized` error rather than a trap.
#[test]
fn test_raise_dispute_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 34);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    // A stranger is neither payer nor payee, so the entrypoint returns
    // `Unauthorized` before it ever reaches `caller.require_auth()`.
    let random = Address::generate(&env);
    assert_eq!(
        client.try_raise_dispute(&random, &id, &0u32),
        Err(Ok(TrellisError::Unauthorized)),
        "a non-party caller must not be able to raise a dispute"
    );
}

/// `cancel_unfunded_milestone` is payer-only: without a payer signature it traps.
#[test]
#[should_panic(expected = "InvalidAction")]
fn test_cancel_unfunded_wrong_role_fails() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 35);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 500),
        &dispute_resolver,
    );

    // `cancel_unfunded_milestone` gates on `agreement.payer.require_auth()`.
    deny_all_auth(&env);
    client.cancel_unfunded_milestone(&id, &0u32);
}

/// Test get_total_amount returns the correct sum of all milestone amounts.
#[test]
fn test_get_total_amount_matches_sum() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 40);

    let milestones = vec![
        &env,
        Milestone {
            amount: 1_000,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 2_500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
        Milestone {
            amount: 1_500,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ];

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &milestones,
        &dispute_resolver,
    );

    // The generated client unwraps the contract-level `Result`, so
    // `get_total_amount` surfaces an `AgreementNotFound` as a test panic rather
    // than a returnable value here.
    let total = client.get_total_amount(&id);
    assert_eq!(
        total, 5_000,
        "get_total_amount should return sum of all milestones"
    );
}

/// Test extend_agreement_ttl on an existing agreement.
#[test]
fn test_extend_ttl_success() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 41);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );

    // extend_agreement_ttl has no require_auth() gate — it's a permissionless
    // keeper entrypoint — so no auth mock is needed here. It reports the
    // keeper address in `ttl_extended`, so pass the caller explicitly.
    let result = client.try_extend_agreement_ttl(&id, &payer);
    assert_eq!(
        result,
        Ok(Ok(())),
        "extend_agreement_ttl should succeed on existing agreement"
    );
}

/// Test extend_agreement_ttl on non-existent agreement fails gracefully.
#[test]
fn test_extend_ttl_nonexistent_agreement() {
    let (env, payer, _payee, _dispute_resolver, _token_address, client) = setup();
    let id = agreement_id(&env, 99);

    // No auth mock needed — see comment above test_extend_ttl_success.
    let result = client.try_extend_agreement_ttl(&id, &payer);
    assert_eq!(
        result,
        Err(Ok(TrellisError::AgreementNotFound)),
        "extend_agreement_ttl on non-existent agreement should return AgreementNotFound"
    );
}

/// Test dispute raised by payer.
#[test]
fn test_dispute_raised_by_payer() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 42);
    let amount: i128 = 2_000;

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, amount),
        &dispute_resolver,
    );

    client.lock_funds(&id, &0u32);

    // Payer raises the dispute
    client.raise_dispute(&payer, &id, &0u32);

    // Verify milestone status transitioned to Disputed
    let milestone = client.get_milestone(&id, &0u32);
    assert_eq!(
        milestone.expect("milestone 0 must still exist").status,
        EscrowStatus::Disputed,
        "milestone should transition to Disputed when payer raises dispute"
    );
}
