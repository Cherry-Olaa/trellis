use soroban_sdk::{symbol_short, Address, BytesN, Env, String};

// ---------------------------------------------------------------------------
// Event emitters — the only place that calls env.events().publish().
//
// Convention:
//   • topics: a tuple of (Symbol, agreement_id) — enables indexed filtering
//     by event name and/or agreement ID from off-chain indexers.
//   • data:   a tuple of typed Soroban primitives (Symbol, i128, Address, u32,
//     bool, Option<String>) so off-chain consumers can decode payloads without
//     importing the contract WASM types.
//
// Symbol length limits:
//   • `symbol_short!` accepts ≤9 characters and stores the symbol as an
//     inline 64-bit integer — no heap allocation on the guest WASM side.
//   • `symbol_short!` is used where the prefixed name is exactly 9 chars or
//     fewer.  Any name that exceeds 9 chars uses `Symbol::new(env, "…")`,
//     which stores the string in the host's symbol table.
//
// Prefixed topic names and their lengths:
//   1. "trls_crte"  ( 9 chars) → symbol_short! — agreement_created  (abbreviated to stay ≤9)
//   2. "trls_lckd"  ( 9 chars) → symbol_short! — funds_locked       (abbreviated)
//   3. "trls_sbmt"  ( 9 chars) → symbol_short! — work_submitted     (abbreviated)
//   4. "trls_rlsd"  ( 9 chars) → symbol_short! — funds_released     (abbreviated)
//   5. "trls_dspt"  ( 9 chars) → symbol_short! — dispute_raised     (abbreviated)
//   6. "trls_rslv"  ( 9 chars) → symbol_short! — milestone_resolved (abbreviated)
//   7. "trls_cncl"  ( 9 chars) → symbol_short! — milestone_cancelled(abbreviated)
//   8. "trls_ttle"  ( 9 chars) → symbol_short! — ttl_extended       (abbreviated)
//   9. "trls_prtl"  ( 9 chars) → symbol_short! — funds_partially_released (abbreviated)
//  10. "trls_cmpl"  ( 9 chars) → symbol_short! — agreement_completed (abbreviated)
//
// All abbreviations follow the pattern: drop the vowels from the root word
// while keeping enough consonants to be unambiguous.  They are intentional
// and documented here so indexers can rely on them as stable identifiers.
//
// Event schema (topics → data):
//   1. "created"   → (payer: Address, payee: Address)
//   2. "locked"    → (milestone_id: u32, amount: i128)
//   3. "submitted" → (milestone_id: u32, proof_uri: Option<String>)
//   4. "released"  → (milestone_id: u32, amount: i128)
//   5. "disputed"  → (milestone_id: u32, caller: Address, amount: i128)
//   6. "resolved"  → (milestone_id: u32, refunded_to_payer: bool, amount: i128)
//   7. "cancelled" → (milestone_id: u32, payer: Address, cancelled_by: Address, amount: i128)
//   8. "ttl_extended" → (caller: Address)
//   9. "partially_released" → (milestone_id: u32, amount: i128, remaining: i128)
//  10. "agreement_completed" → (completed_count: u32, refunded_count: u32)
//
// Schema changes: the `amount` field on "disputed", "resolved" and
// "cancelled" was appended as the last tuple element, so an indexer that
// decodes those payloads positionally keeps reading the earlier fields at
// the same indices. Every fund-movement event now carries an amount.
//
// `amount` semantics: on "released", "disputed" and "resolved" it is the
// amount still held in escrow for the milestone at the time of the call —
// the milestone's configured `amount` minus anything already paid out via
// `release_partial`. With no partial releases this equals the milestone's
// stored amount. On "cancelled" it is the proposed (never funded) amount.
//
// "trls_rslv" and "trls_cncl" are deliberately distinct: an indexer must be
// able to tell an arbitrated dispute outcome apart from a payer walking back a
// milestone that was never funded, since only the former involves token
// movement and resolver involvement.
// ---------------------------------------------------------------------------

/// Emitted when a new escrow agreement is created.
///
/// Topics: `("trls_crte", agreement_id)`
/// Data:   `(payer, payee)`
pub fn agreement_created(env: &Env, agreement_id: BytesN<32>, payer: Address, payee: Address) {
    env.events().publish(
        (symbol_short!("trls_crte"), agreement_id.clone()),
        (payer, payee),
    );
}

/// Emitted when funds for a specific milestone are locked into the escrow.
///
/// Topics: `("trls_lckd", agreement_id)`
/// Data:   `(milestone_id, amount)`
pub fn funds_locked(env: &Env, agreement_id: BytesN<32>, milestone_id: u32, amount: i128) {
    env.events().publish(
        (symbol_short!("trls_lckd"), agreement_id.clone()),
        (milestone_id, amount),
    );
}

/// Emitted when a payee submits proof of work for a milestone.
///
/// Topics: `("trls_sbmt", agreement_id)`
/// Data:   `(milestone_id, proof_uri)`
///
/// `proof_uri` is `None` when the milestone was submitted without a proof
/// link; it is never an empty string.
pub fn work_submitted(
    env: &Env,
    agreement_id: BytesN<32>,
    milestone_id: u32,
    proof_uri: Option<String>,
) {
    env.events().publish(
        (symbol_short!("trls_sbmt"), agreement_id.clone()),
        (milestone_id, proof_uri),
    );
}

/// Emitted when a payer approves a milestone and funds are released to the payee.
///
/// Topics: `("trls_rlsd", agreement_id)`
/// Data:   `(milestone_id, amount)`
///
/// `amount` is what this call actually transferred: the milestone's amount
/// minus anything already paid out through [`funds_partially_released`].
pub fn funds_released(env: &Env, agreement_id: BytesN<32>, milestone_id: u32, amount: i128) {
    env.events().publish(
        (symbol_short!("trls_rlsd"), agreement_id.clone()),
        (milestone_id, amount),
    );
}

/// Emitted when either party raises a dispute on a funded or work-submitted milestone.
///
/// Topics: `("trls_dspt", agreement_id)`
/// Data:   `(milestone_id, caller, amount)`
///
/// `caller` is the party (payer or payee) that triggered the dispute, provided
/// as a typed `Address` so indexers can identify the initiating party without
/// importing contract WASM types.
///
/// `amount` is the escrowed amount at stake in the dispute — the milestone's
/// amount minus anything already partially released — so a dispute dashboard
/// does not need to cross-reference the earlier `funds_locked` event.
pub fn dispute_raised(
    env: &Env,
    agreement_id: BytesN<32>,
    milestone_id: u32,
    caller: Address,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("trls_dspt"), agreement_id.clone()),
        (milestone_id, caller, amount),
    );
}

/// Emitted when the dispute resolver settles a disputed milestone.
///
/// This event means an arbitration ruling was made and tokens moved. It is
/// **not** emitted for cancellations — see [`milestone_cancelled`].
///
/// Topics: `("trls_rslv", agreement_id)`
/// Data:   `(milestone_id, refunded_to_payer, amount)`
///
/// `refunded_to_payer = true`  → locked funds returned to payer.
/// `refunded_to_payer = false` → locked funds awarded to payee.
///
/// `amount` is the escrowed amount the ruling moved — the milestone's amount
/// minus anything already partially released before the dispute.
pub fn milestone_resolved(
    env: &Env,
    agreement_id: BytesN<32>,
    milestone_id: u32,
    refunded_to_payer: bool,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("trls_rslv"), agreement_id.clone()),
        (milestone_id, refunded_to_payer, amount),
    );
}

/// Emitted when a payer cancels a milestone that was never funded.
///
/// No tokens move: the milestone simply leaves the `Pending` state without
/// ever having been escrowed. Indexers should treat this as a withdrawal of a
/// proposal, not as a dispute outcome.
///
/// Topics: `("trls_cncl", agreement_id)`
/// Data:   `(milestone_id, payer, cancelled_by, amount)`
///
/// `payer` is the agreement's payer; `cancelled_by` is the address that
/// authorised the cancellation. These are the same address today (only the
/// payer may cancel) but are reported separately so the event stays
/// self-describing if delegated cancellation is ever added.
///
/// `amount` is the milestone's proposed amount. It was never escrowed, so no
/// tokens move — it lets an indexer show "a $X milestone was withdrawn"
/// without a separate query. Any future cancellation-adjacent event (e.g. a
/// payee-side withdrawal) should carry the same trailing `amount` field.
pub fn milestone_cancelled(
    env: &Env,
    agreement_id: BytesN<32>,
    milestone_id: u32,
    payer: Address,
    cancelled_by: Address,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("trls_cncl"), agreement_id.clone()),
        (milestone_id, payer, cancelled_by, amount),
    );
}

/// Emitted when the payer releases part of a milestone's escrowed amount to
/// the payee ahead of (or instead of) full approval.
///
/// Topics: `("trls_prtl", agreement_id)`
/// Data:   `(milestone_id, amount, remaining)`
///
/// `amount` is what this call transferred; `remaining` is what is still held
/// in escrow for the milestone afterwards. `remaining == 0` means the
/// milestone moved to `Completed` in the same call.
pub fn funds_partially_released(
    env: &Env,
    agreement_id: BytesN<32>,
    milestone_id: u32,
    amount: i128,
    remaining: i128,
) {
    env.events().publish(
        (symbol_short!("trls_prtl"), agreement_id.clone()),
        (milestone_id, amount, remaining),
    );
}

/// Emitted once per agreement, when its last non-terminal milestone reaches a
/// terminal state.
///
/// "Completed" here means *settled*: every milestone is either `Completed`
/// (paid to the payee, by approval, full partial release, or a ruling for
/// the payee) or `Refunded` (returned to the payer by a ruling, or cancelled
/// before funding). No milestone can transition again, and the agreement
/// holds no escrowed funds. It does **not** mean every milestone was paid
/// out — use the counts to tell a fully delivered agreement from one that
/// was partly or wholly refunded.
///
/// Topics: `("trls_cmpl", agreement_id)`
/// Data:   `(completed_count, refunded_count)`
///
/// It is emitted after the milestone-level event of the transition that
/// triggered it, in the same invocation.
pub fn agreement_completed(
    env: &Env,
    agreement_id: BytesN<32>,
    completed_count: u32,
    refunded_count: u32,
) {
    env.events().publish(
        (symbol_short!("trls_cmpl"), agreement_id.clone()),
        (completed_count, refunded_count),
    );
}

/// Emitted when a keeper extends the TTL of an agreement to prevent expiry.
///
/// No state is modified beyond the ledger TTL. This event creates an audit
/// trail for off-chain monitoring of keeper activity.
///
/// Topics: `("trls_ttle", agreement_id)`
/// Data:   `(caller)`
pub fn ttl_extended(env: &Env, agreement_id: BytesN<32>, caller: Address) {
    env.events().publish(
        (symbol_short!("trls_ttle"), agreement_id.clone()),
        (caller,),
    );
}
