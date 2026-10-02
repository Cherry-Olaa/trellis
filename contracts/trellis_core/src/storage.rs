use soroban_sdk::{contracttype, BytesN, Env, Vec};

use crate::errors::TrellisError;
use crate::types::{Agreement, AgreementHeader, Milestone};

// ---------------------------------------------------------------------------
// DataKey — typed namespace for every ledger entry written by this contract.
// Using an enum prevents accidental key collisions as the contract grows.
// ---------------------------------------------------------------------------
#[contracttype]
pub enum DataKey {
    /// Persistent storage key for an agreement's *header*: every field of the
    /// agreement except its milestone vector.
    /// The inner BytesN<32> is the globally unique agreement ID.
    Agreement(BytesN<32>),
    /// Persistent storage key for one milestone of an agreement, addressed by
    /// the agreement ID and the milestone's index within it.
    ///
    /// Splitting the milestones out of the header (#401) is what keeps a
    /// single-milestone state transition an O(1) ledger write: `lock_funds`,
    /// `submit_work` and friends rewrite only this entry instead of
    /// re-serialising the whole `Vec<Milestone>` back to storage.
    Milestone(BytesN<32>, u32),
}

// ---------------------------------------------------------------------------
// TTL (rent) parameters
//
// Every persistent ledger entry on Soroban carries a finite time-to-live.
// When the TTL runs out the entry is archived and, without a restore, the data
// is effectively lost.  The TTL does *not* renew itself on write — the
// contract must explicitly call `extend_ttl`.
// ---------------------------------------------------------------------------

/// Ledgers closed in roughly one day, at Stellar's ~5-second close cadence
/// (86_400 / 5 = 17_280).  Used only to express the constants below in
/// human-readable units.
const DAY_IN_LEDGERS: u32 = 17_280;

/// Target lifetime for an agreement entry: ~30 days from the most recent
/// extension.  Every extension resets the entry's TTL to this value.
///
/// This sits well inside Soroban's maximum persistent entry TTL, so the
/// `extend_ttl` call can never be rejected for overshooting the cap.
pub const LEDGER_BUMP: u32 = DAY_IN_LEDGERS * 30;

/// Only pay for an extension once the remaining TTL drops below ~15 days.
///
/// Above this threshold `extend_ttl` is a no-op, so back-to-back writes in the
/// same period do not repeatedly charge rent.  The 15-day gap between the
/// threshold and [`LEDGER_BUMP`] is the window a keeper service has to call
/// `extend_agreement_ttl` on an otherwise idle agreement before it expires.
///
/// `pub` so `test_storage` can age entries past the threshold and assert that a
/// single-milestone transition only re-extends the one entry it wrote (#401).
pub const LEDGER_THRESHOLD: u32 = DAY_IN_LEDGERS * 15;

// ---------------------------------------------------------------------------
// Storage helpers — the only place in the codebase that touches
// env.storage().persistent().  All other modules go through these functions.
// ---------------------------------------------------------------------------

/// Reset the TTL of the single ledger entry `key` to [`LEDGER_BUMP`] ledgers.
///
/// A no-op while the entry still has more than [`LEDGER_THRESHOLD`] ledgers
/// left, and a no-op if the entry does not exist.
fn bump_ttl(env: &Env, key: &DataKey) {
    // extend_ttl traps on a missing entry, so guard the lookup.
    if !env.storage().persistent().has(key) {
        return;
    }

    env.storage()
        .persistent()
        .extend_ttl(key, LEDGER_THRESHOLD, LEDGER_BUMP);
}

/// Reset the TTL of every entry backing agreement `id` — the header plus each
/// of its `milestone_count` milestone entries.
///
/// Only [`extend_agreement_ttl`] needs this: an agreement is now several ledger
/// entries, so a keeper renewing an idle agreement has to keep the milestone
/// entries alive too, not just the header. Every other path bumps only the
/// entry it actually touched.
fn bump_agreement_ttls(env: &Env, id: &BytesN<32>, milestone_count: u32) {
    bump_ttl(env, &DataKey::Agreement(id.clone()));

    for milestone_id in 0..milestone_count {
        bump_ttl(env, &DataKey::Milestone(id.clone(), milestone_id));
    }
}

/// Persist a freshly-initialised [`Agreement`] as one header entry plus one
/// entry per milestone.
///
/// Only `init` calls this. Every later state transition goes through
/// [`write_milestone`], which touches exactly one entry — see the
/// `DataKey::Milestone` docs and issue #401 for why that matters for fees.
///
/// Writing alone does **not** renew an entry's TTL, so each write is followed
/// by a [`bump_ttl`] call. Routing both the header and every milestone through
/// helpers that bump guarantees no state-mutating entrypoint can leave an
/// entry to expire.
///
/// # Errors
/// Returns [`TrellisError::InvalidMilestone`] if the agreement's milestone
/// vector is internally inconsistent — an index it claims to have is missing.
pub fn write_agreement(
    env: &Env,
    id: &BytesN<32>,
    agreement: &Agreement,
) -> Result<(), TrellisError> {
    let header = AgreementHeader {
        agreement_id: agreement.agreement_id.clone(),
        payer: agreement.payer.clone(),
        payee: agreement.payee.clone(),
        token: agreement.token.clone(),
        dispute_resolver: agreement.dispute_resolver.clone(),
        total_amount: agreement.total_amount,
        milestone_count: agreement.milestones.len(),
        released_amounts: agreement.released_amounts.clone(),
    };

    let header_key = DataKey::Agreement(id.clone());
    env.storage().persistent().set(&header_key, &header);
    bump_ttl(env, &header_key);

    for milestone_id in 0..header.milestone_count {
        let milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;
        write_milestone(env, id, milestone_id, &milestone);
    }

    Ok(())
}

/// Read only an agreement's header — one ledger read, regardless of how many
/// milestones the agreement has.
///
/// This is what makes each milestone state transition O(1): an entrypoint that
/// only needs the parties, the token or the resolver never deserialises the
/// milestone list at all.
///
/// # Errors
/// Returns [`TrellisError::AgreementNotFound`] when no agreement exists for
/// `id`. Deliberately does not touch the milestone entries — see
/// [`bump_agreement_ttls`].
pub fn read_header(env: &Env, id: &BytesN<32>) -> Result<AgreementHeader, TrellisError> {
    let key = DataKey::Agreement(id.clone());
    let header: AgreementHeader = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(TrellisError::AgreementNotFound)?;

    // Reaching here proves the entry exists, so the bump is safe.
    bump_ttl(env, &key);

    Ok(header)
}

/// Persist an updated [`AgreementHeader`] — one ledger write.
///
/// Only [`crate::lib`] uses this, and only when it has to change
/// agreement-level data: today that is `release_partial` recording a
/// cumulative release in `released_amounts`. Milestone status transitions
/// never call it — they go through [`write_milestone`] — which is what keeps
/// them to a single ledger write (#401).
pub fn write_header(env: &Env, id: &BytesN<32>, header: &AgreementHeader) {
    let key = DataKey::Agreement(id.clone());
    env.storage().persistent().set(&key, header);
    bump_ttl(env, &key);
}

/// Read a single milestone — one ledger read, regardless of agreement size.
///
/// # Errors
/// Returns [`TrellisError::InvalidMilestone`] (code 4) both when `milestone_id`
/// is out of range and when the entry is otherwise absent. That matches the
/// pre-split behaviour, where both cases were a single `Vec::get(..)` miss, so
/// no entrypoint changed the error it reports.
pub fn read_milestone(
    env: &Env,
    id: &BytesN<32>,
    milestone_id: u32,
) -> Result<Milestone, TrellisError> {
    let key = DataKey::Milestone(id.clone(), milestone_id);
    let milestone: Milestone = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(TrellisError::InvalidMilestone)?;

    bump_ttl(env, &key);

    Ok(milestone)
}

/// Write a single milestone — one ledger write, regardless of agreement size.
///
/// Every state-mutating entrypoint ends up here, so a transition on milestone
/// *k* can never rewrite milestones *j != k*, nor the header.
pub fn write_milestone(env: &Env, id: &BytesN<32>, milestone_id: u32, milestone: &Milestone) {
    let key = DataKey::Milestone(id.clone(), milestone_id);
    env.storage().persistent().set(&key, milestone);
    bump_ttl(env, &key);
}

/// Renew the TTL of an existing agreement without modifying it.
///
/// Backs the public `extend_agreement_ttl` entrypoint so keeper services can
/// keep long-running agreements alive between state transitions. Because an
/// agreement is now several ledger entries, this renews *all* of them — a
/// keeper that only renewed the header would eventually strand orphaned
/// milestone entries.
///
/// # Errors
/// Returns [`TrellisError::AgreementNotFound`] if no entry exists for `id`,
/// so a keeper pointed at a bad ID fails loudly instead of silently no-opping.
pub fn extend_agreement_ttl(env: &Env, id: &BytesN<32>) -> Result<(), TrellisError> {
    let header = read_header(env, id)?;

    bump_agreement_ttls(env, id, header.milestone_count);

    Ok(())
}

/// Retrieve the full [`Agreement`] for `id`, reassembled from its header and
/// per-milestone entries.
///
/// This is the only O(n) path in storage, and it backs the read-only views
/// (`get_agreement`) that genuinely need every milestone. State transitions do
/// not go through it — they read a header plus one milestone.
///
/// Reading also refreshes each entry's TTL. An agreement that is merely being
/// watched — a long dispute window, a milestone awaiting delivery — is still
/// in active use and must not be archived out from under its parties.
///
/// # Errors
/// Returns [`TrellisError::AgreementNotFound`] (code 3) when no record exists
/// for `id`. Callers may also use [`has_agreement`] to pre-check existence.
pub fn read_agreement(env: &Env, id: &BytesN<32>) -> Result<Agreement, TrellisError> {
    let header = read_header(env, id)?;

    let mut milestones: Vec<Milestone> = Vec::new(env);
    for milestone_id in 0..header.milestone_count {
        milestones.push_back(read_milestone(env, id, milestone_id)?);
    }

    Ok(Agreement {
        agreement_id: header.agreement_id,
        payer: header.payer,
        payee: header.payee,
        token: header.token,
        milestones,
        dispute_resolver: header.dispute_resolver,
        total_amount: header.total_amount,
        released_amounts: header.released_amounts,
    })
}

/// Return `true` if an [`Agreement`] with the given `id` exists in storage.
///
/// Deliberately does not bump the TTL: this is a pure existence probe used by
/// `init` to reject duplicate IDs, and it must not charge rent for asking
/// about an agreement that may not exist.
pub fn has_agreement(env: &Env, id: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Agreement(id.clone()))
}
