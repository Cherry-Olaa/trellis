//! Storage-layout tests for the Trellis contract (#401).
//!
//! #401 split an agreement's milestone vector out of its single ledger entry so
//! that a one-milestone state transition writes exactly one entry instead of
//! re-serialising every milestone. These tests pin that down at the ledger
//! level rather than only through the contract's return values: a stray
//! whole-agreement write, or a milestone stored under the wrong index, still
//! looks correct to a test that only inspects `get_agreement`.
//!
//! The technique is entry-level. Each test reads what is actually stored under
//! a [`DataKey`] (via `env.as_contract`, since ledger storage is unreachable
//! from outside a contract frame) and asserts exactly which entries a
//! transition touched. Because the header no longer carries the milestone
//! vector, a wholesale re-write of "the agreement" is not merely avoided but
//! unrepresentable — and if a change reintroduced a header write, the header
//! would differ here.

use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _},
    token, Address, BytesN, Env, Vec,
};

use crate::{
    storage::{DataKey, LEDGER_BUMP},
    types::{AgreementHeader, EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

/// Local fixture.
///
/// #403 consolidates the helpers in `test.rs` / `test_properties.rs` /
/// `test_panic_boundaries.rs` onto a shared `test_utils`; this module predates
/// that in the dependency order, so it keeps a local copy rather than making
/// #401 depend on an unrelated refactor.
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
    token::StellarAssetClient::new(&env, &token_address).mint(&payer, &1_000_000_000);

    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

/// Build a 32-byte agreement ID from a seed byte.
fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Three Pending milestones of distinct sizes, so a mixed-up index would be
/// visible in both status and amount.
fn three_milestones(env: &Env) -> Vec<Milestone> {
    let amounts: [i128; 3] = [1_000, 2_000, 3_000];
    let mut milestones: Vec<Milestone> = Vec::new(env);
    for amount in amounts {
        milestones.push_back(Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
            // `split` records a dispute outcome; these fixtures never reach a
            // dispute, so it stays `None`.
            split: None,
        });
    }
    milestones
}

/// The [`Milestone`] stored under `DataKey::Milestone`.
fn milestone_entry(env: &Env, contract: &Address, id: &BytesN<32>, milestone_id: u32) -> Milestone {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(id.clone(), milestone_id))
            .expect("per-milestone entry must exist")
    })
}

/// The [`AgreementHeader`] stored under `DataKey::Agreement`.
fn header_entry(env: &Env, contract: &Address, id: &BytesN<32>) -> AgreementHeader {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get(&DataKey::Agreement(id.clone()))
            .expect("header entry must exist")
    })
}

/// `Milestone` deliberately has no `PartialEq` (#399 tracks adding one), so
/// compare the fields a transition could plausibly disturb.
fn assert_same_milestone(actual: &Milestone, expected: &Milestone, label: &str) {
    assert_eq!(actual.amount, expected.amount, "{label}: amount");
    assert_eq!(actual.status, expected.status, "{label}: status");
    assert_eq!(actual.proof_uri, expected.proof_uri, "{label}: proof_uri");
}

/// #401 — after `init`, every milestone owns its own ledger entry and the
/// agreement entry holds only the header.
///
/// This is the structural half of the fix: the old layout put the whole
/// `Vec<Milestone>` behind `DataKey::Agreement`, which is exactly what made a
/// one-milestone transition cost O(n) writes.
#[test]
fn test_init_stores_one_entry_per_milestone_plus_a_header() {
    let (env, payer, payee, resolver, token_address, client) = setup();
    let id = agreement_id(&env, 60);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &three_milestones(&env),
        &resolver,
    );

    let contract = client.address.clone();
    let header = header_entry(&env, &contract, &id);
    assert_eq!(
        header.milestone_count, 3,
        "header records the milestone count"
    );
    assert_eq!(
        header.total_amount, 6_000,
        "header keeps the pre-computed total"
    );

    for (milestone_id, amount) in [1_000i128, 2_000, 3_000].into_iter().enumerate() {
        let stored = milestone_entry(&env, &contract, &id, milestone_id as u32);
        assert_eq!(stored.amount, amount, "milestone {milestone_id} amount");
        assert_eq!(stored.status, EscrowStatus::Pending);
    }

    assert!(
        env.as_contract(&contract, || {
            env.storage()
                .persistent()
                .get::<_, Milestone>(&DataKey::Milestone(id.clone(), 3))
                .is_none()
        }),
        "an agreement must not own entries past its milestone count"
    );
}

/// #401 — a transition on milestone *k* must leave every other milestone entry
/// *and* the header untouched.
///
/// Before the split, `lock_funds` re-serialised the whole `Vec<Milestone>` into
/// the single `DataKey::Agreement` entry, so a transition on one milestone of a
/// long agreement paid write cost — and rent — for all of them.
#[test]
fn test_single_milestone_transition_rewrites_only_that_milestone_entry() {
    let (env, payer, payee, resolver, token_address, client) = setup();
    let id = agreement_id(&env, 61);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &three_milestones(&env),
        &resolver,
    );

    let contract = client.address.clone();
    let header_before = header_entry(&env, &contract, &id);
    let neighbour_0 = milestone_entry(&env, &contract, &id, 0);
    let neighbour_2 = milestone_entry(&env, &contract, &id, 2);

    // Transition ONE milestone in the middle of the agreement.
    client.lock_funds(&id, &1u32);

    assert_same_milestone(
        &milestone_entry(&env, &contract, &id, 0),
        &neighbour_0,
        "milestone 0",
    );
    assert_same_milestone(
        &milestone_entry(&env, &contract, &id, 2),
        &neighbour_2,
        "milestone 2",
    );

    let header_after = header_entry(&env, &contract, &id);
    assert_eq!(header_after.milestone_count, header_before.milestone_count);
    assert_eq!(header_after.total_amount, header_before.total_amount);
    assert_eq!(header_after.agreement_id, header_before.agreement_id);
    assert_eq!(header_after.payer, header_before.payer);
    assert_eq!(header_after.payee, header_before.payee);
    assert_eq!(header_after.token, header_before.token);
    assert_eq!(
        header_after.dispute_resolver, header_before.dispute_resolver,
        "a transition must not rewrite the agreement header"
    );

    assert_eq!(
        milestone_entry(&env, &contract, &id, 1).status,
        EscrowStatus::Funded,
        "the transitioned milestone's entry must have been rewritten"
    );
}

/// Adjacent case — the legitimate happy path still works after the split:
/// `lock_funds` moves exactly the targeted milestone's funds, leaves its
/// neighbours untouched, and `get_agreement` reassembles every milestone from
/// the per-milestone entries.
#[test]
fn test_happy_path_after_split_reassembles_every_milestone() {
    let (env, payer, payee, resolver, token_address, client) = setup();
    let id = agreement_id(&env, 62);
    let token_client = token::TokenClient::new(&env, &token_address);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &three_milestones(&env),
        &resolver,
    );

    client.lock_funds(&id, &1u32);

    assert_eq!(
        token_client.balance(&client.address),
        2_000,
        "only the locked milestone's amount should have moved into the contract"
    );

    let agreement = client.get_agreement(&id);
    assert_eq!(
        agreement.milestones.len(),
        3,
        "every milestone must reassemble"
    );
    assert_eq!(
        agreement.total_amount, 6_000,
        "header keeps the pre-computed total"
    );
    assert_eq!(agreement.payer, payer);
    assert_eq!(agreement.payee, payee);
    assert_eq!(agreement.dispute_resolver, resolver);

    let milestone_0 = agreement.milestones.get(0).unwrap();
    let milestone_1 = agreement.milestones.get(1).unwrap();
    let milestone_2 = agreement.milestones.get(2).unwrap();
    assert_eq!(milestone_0.status, EscrowStatus::Pending);
    assert_eq!(milestone_1.status, EscrowStatus::Funded);
    assert_eq!(milestone_1.amount, 2_000);
    assert_eq!(milestone_2.status, EscrowStatus::Pending);

    // The per-milestone view is still a single read and still agrees.
    assert_eq!(
        client.get_milestone(&id, &2u32).map(|m| m.status),
        Some(EscrowStatus::Pending)
    );
    assert!(
        client.get_milestone(&id, &9u32).is_none(),
        "out-of-range index reads nothing"
    );

    // `extend_agreement_ttl` renews every entry of the now-split agreement, not
    // just the header — otherwise a keeper would strand milestone entries.
    client.extend_agreement_ttl(&id, &payer);
    let contract = client.address.clone();
    for milestone_id in 0..3 {
        assert_eq!(
            env.as_contract(&contract, || {
                env.storage()
                    .persistent()
                    .get_ttl(&DataKey::Milestone(id.clone(), milestone_id))
            }),
            LEDGER_BUMP,
            "keeper renewal must cover milestone {milestone_id}"
        );
    }
}
