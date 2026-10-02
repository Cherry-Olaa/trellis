//! Shared fixtures for the contract's three test suites (#403).
//!
//! `test.rs`, `test_properties.rs` and `test_panic_boundaries.rs` all need the
//! same things: a deterministic agreement ID, a fresh Soroban [`Env`] with a
//! funded payer and a registered Trellis contract, and small builders that turn
//! plain amounts into `Vec<Milestone>`. Each file used to carry its own copy,
//! so a tweak to one suite's setup could silently stop applying to the others
//! and the suites would drift apart. There is now exactly one implementation of
//! each helper, here, and the suites import it.
//!
//! The suites legitimately differ in two ways, which the two `setup*` fns
//! below encode instead of duplicating:
//!
//! | suite | payer mint | auth mocking |
//! |-------|------------|--------------|
//! | `test.rs` | [`EXAMPLE_MINT`] | none — each step installs its own [`auth_as`] mock so role checks are exercised against real signatures |
//! | `test_properties.rs`, `test_panic_boundaries.rs` | [`FUZZ_MINT`] | blanket `env.mock_all_auths()`, because those suites drive every entrypoint from arbitrary callers and assert typed contract errors rather than signature gates |
//!
//! Both variants share [`deploy`], so the token/contract wiring can never
//! diverge between them.

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, vec, Address, BytesN, Env, Vec,
};

use crate::{
    types::{EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

// ---------------------------------------------------------------------------
// Fixture parameters
// ---------------------------------------------------------------------------

/// Payer mint used by the example-based tests in `test.rs`.
///
/// Deliberately small: `test_batch_lock_funds` asserts an absolute end balance
/// for the payer (10_000 − 1_000 = 9_000), so changing this changes that test.
pub const EXAMPLE_MINT: i128 = 10_000;

/// Payer mint used by the property-based and panic-boundary suites.
///
/// These generate multi-milestone agreements whose amounts reach 100_000 each,
/// so the payer needs a balance that comfortably covers any generated set.
pub const FUZZ_MINT: i128 = 1_000_000_000;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build a 32-byte agreement ID from a seed byte.
pub fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Authenticate a specific address for testing.
/// Replaces blanket `env.mock_all_auths()` with granular per-caller auth.
pub fn auth_as(env: &Env, address: &Address) {
    env.mock_auths(&[MockAuth {
        address,
        invoke: &MockAuthInvoke {
            contract: address,
            fn_name: "",
            args: vec![env],
            sub_invokes: &[],
        },
    }]);
}

/// Create a single `Milestone` with the given amount, starting life `Pending`.
pub fn one_milestone(env: &Env, amount: i128) -> Vec<Milestone> {
    vec![
        env,
        Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ]
}

/// Build a `Vec<Milestone>` from a slice of amounts. All statuses are Pending.
///
/// Idx of each milestone is its position in `amounts`, matching the contract's
/// index-addressed milestone lookups.
pub fn milestones_from_amounts(env: &Env, amounts: &[i128]) -> Vec<Milestone> {
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
// Setup
// ---------------------------------------------------------------------------

/// The single deployment routine both fixtures delegate to.
///
/// `env` is passed in rather than created here so a fixture can install its
/// auth policy *before* the SAC `mint` (which needs the admin's signature) —
/// that ordering is the only thing the two fixtures differ on besides the mint
/// amount.
///
/// Returns `(env, payer, payee, dispute_resolver, token_address, client)`.
fn deploy(
    env: Env,
    mint_amount: i128,
) -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let dispute_resolver = Address::generate(&env);

    // Deploy the built-in Stellar Asset Contract and mint payer a balance.
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    token_admin_client.mint(&payer, &mint_amount);

    // Register the Trellis contract.
    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

/// Common example-based test fixture used by `test.rs`.
///
/// Returns `(env, payer, payee, dispute_resolver, token_address, client)`.
/// **Note**: auth is NOT mocked by default — tests must call [`auth_as`] explicitly.
pub fn setup() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    deploy(Env::default(), EXAMPLE_MINT)
}

/// Fixture used by `test_properties.rs` and `test_panic_boundaries.rs`.
///
/// Same wiring as [`setup`] but funds the payer generously ([`FUZZ_MINT`]) and
/// blanket-mocks auth, so a property test can call any entrypoint from any
/// generated caller without wiring a per-step mock.
pub fn setup_mocked() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    TrellisContractClient<'static>,
) {
    let env = Env::default();
    // Installed before `deploy` so it covers the SAC mint below.
    env.mock_all_auths();
    deploy(env, FUZZ_MINT)
}
