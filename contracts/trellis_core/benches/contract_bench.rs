// Soroban gas cost benchmarks for all contract entrypoints.
//
// Run with:
//   cargo bench --bench contract_bench
//
// Output is machine-parseable JSON for CI comparison.

use soroban_sdk::{
    testutils::Address as _, token, vec, Address, BytesN, Env, String as SorobanString,
    Vec as SorobanVec,
};
use trellis_core::{
    types::{EscrowStatus, Milestone},
    TrellisContract, TrellisContractClient,
};

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn agreement_id(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn one_milestone(env: &Env, amount: i128) -> SorobanVec<Milestone> {
    vec![
        env,
        Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
        },
    ]
}

fn n_milestones(env: &Env, count: u32, amount: i128) -> SorobanVec<Milestone> {
    let mut v: SorobanVec<Milestone> = SorobanVec::new(env);
    for _ in 0..count {
        v.push_back(Milestone {
            amount,
            status: EscrowStatus::Pending,
            proof_uri: None,
        });
    }
    v
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
    env.mock_all_auths();

    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let dispute_resolver = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    token_admin_client.mint(&payer, &1_000_000_000);

    let contract_id = env.register(TrellisContract, ());
    let client = TrellisContractClient::new(&env, &contract_id);

    (env, payer, payee, dispute_resolver, token_address, client)
}

// ---------------------------------------------------------------------------
// Benchmark functions
// ---------------------------------------------------------------------------

fn bench_init_1_milestone() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 1);

    env.cost_estimate().budget().reset_default();

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"init_1_milestone","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_init_10_milestones() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 2);

    env.cost_estimate().budget().reset_default();

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &n_milestones(&env, 10, 1_000),
        &dispute_resolver,
    );

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"init_10_milestones","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_lock_funds() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 3);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );

    env.cost_estimate().budget().reset_default();

    client.lock_funds(&id, &0u32);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"lock_funds","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_submit_work() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 4);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );
    client.lock_funds(&id, &0u32);

    env.cost_estimate().budget().reset_default();

    let proof = Some(SorobanString::from_str(&env, "ipfs://test"));
    client.submit_work(&id, &0u32, &proof);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"submit_work","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_approve_and_release() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 5);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );
    client.lock_funds(&id, &0u32);
    client.submit_work(&id, &0u32, &None);

    env.cost_estimate().budget().reset_default();

    client.approve_and_release(&id, &0u32);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"approve_and_release","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_raise_dispute() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 6);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );
    client.lock_funds(&id, &0u32);

    env.cost_estimate().budget().reset_default();

    client.raise_dispute(&payer, &id, &0u32);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"raise_dispute","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_resolve_dispute() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 7);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &one_milestone(&env, 1_000),
        &dispute_resolver,
    );
    client.lock_funds(&id, &0u32);
    client.raise_dispute(&payer, &id, &0u32);

    env.cost_estimate().budget().reset_default();

    client.resolve_dispute(&id, &0u32, &true);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"resolve_dispute","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_batch_lock_funds() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 8);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &n_milestones(&env, 5, 1_000),
        &dispute_resolver,
    );

    env.cost_estimate().budget().reset_default();

    let milestone_ids = vec![&env, 0u32, 1u32, 2u32, 3u32, 4u32];
    client.batch_lock_funds(&id, &milestone_ids);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"batch_lock_funds_5","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

fn bench_get_agreement() {
    let (env, payer, payee, dispute_resolver, token_address, client) = setup();
    let id = agreement_id(&env, 9);

    client.init(
        &id,
        &payer,
        &payee,
        &token_address,
        &n_milestones(&env, 3, 1_000),
        &dispute_resolver,
    );

    env.cost_estimate().budget().reset_default();

    client.get_agreement(&id);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    println!(
        r#"{{"benchmark":"get_agreement","cpu_instructions":{},"memory_bytes":{}}}"#,
        cpu, mem
    );
}

// ---------------------------------------------------------------------------
// Main benchmark runner
// ---------------------------------------------------------------------------

fn main() {
    println!("[");
    bench_init_1_milestone();
    println!(",");
    bench_init_10_milestones();
    println!(",");
    bench_lock_funds();
    println!(",");
    bench_submit_work();
    println!(",");
    bench_approve_and_release();
    println!(",");
    bench_raise_dispute();
    println!(",");
    bench_resolve_dispute();
    println!(",");
    bench_batch_lock_funds();
    println!(",");
    bench_get_agreement();
    println!("]");
}
