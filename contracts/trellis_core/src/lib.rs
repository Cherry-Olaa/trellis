#![no_std]

mod errors;
mod events;
mod storage;
pub mod types;

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_properties;

#[cfg(test)]
mod test_panic_boundaries;

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Map, String, Vec};

use errors::TrellisError;
use types::{Agreement, EscrowStatus, Milestone};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_MILESTONES: u32 = 50;

// ---------------------------------------------------------------------------
// Contract struct
// ---------------------------------------------------------------------------

#[contract]
pub struct TrellisContract;

// ---------------------------------------------------------------------------
// Contract entrypoints
// ---------------------------------------------------------------------------
//
// Panic safety (#154): a panic in a Soroban contract traps the host — the
// transaction reverts but the caller is still charged the fee, and an
// unexpected trap can leave callers reasoning about inconsistent state. Every
// entrypoint below is therefore panic-free by construction:
//
//   * milestone lookups use `Vec::get(id).ok_or(TrellisError::InvalidMilestone)?`
//     — never `Vec::get(id).unwrap()` or index syntax;
//   * agreement reads go through `storage::read_agreement`, which maps a
//     missing entry to `TrellisError::AgreementNotFound` via `Option::ok_or`;
//   * every fallible entrypoint returns `Result<_, TrellisError>` so failures
//     propagate as typed on-chain errors, not panics.
//
// There are no `unwrap()` / `expect()` calls anywhere in the contract crate's
// non-test sources. The `token::Client` transfer calls can still trap inside
// the SDK (e.g. the payer does not hold enough balance) — that is the token
// contract's own boundary and is intentionally left to it. A custom
// `#[panic_handler]` is not added: `soroban-sdk` already provides one for the
// wasm build and a second definition is a duplicate-lang-item error.
// `test_panic_boundaries.rs` fuzzes these paths to keep the property enforced.

#[contractimpl]
impl TrellisContract {
    /// Create a new escrow agreement.
    ///
    /// The payer authorises this call.  Every milestone in `milestones` must
    /// arrive with `status = EscrowStatus::Pending` — the contract rejects any
    /// other initial status with
    /// [`TrellisError::InvalidInitialMilestoneStatus`] rather than silently
    /// accepting it.
    ///
    /// `Pending` is the only valid starting state because it is the sole
    /// entry point of the state machine: `Funded` is reached by
    /// `lock_funds`, and every status after that is reached by transitioning
    /// a funded milestone. All agreements using a given token share one
    /// pooled contract balance, so a milestone created in a pre-advanced
    /// state would be a claim on tokens that were never escrowed for it —
    /// `WorkSubmitted` could go straight to `approve_and_release` and
    /// `Disputed` straight to `resolve_dispute`, either draining the pool
    /// without anything having been locked.
    ///
    /// `milestones` must be non-empty and `dispute_resolver` must be distinct
    /// from both `payer` and `payee` — see the `Errors` below.
    ///
    /// # Errors
    /// - [`TrellisError::AlreadyInitialized`] if an agreement with this ID
    ///   already exists in storage.
    /// - [`TrellisError::EmptyMilestoneSet`] if `milestones` is empty — such
    ///   an agreement could never transition through any state.
    /// - [`TrellisError::InvalidInitialMilestoneStatus`] if any milestone's
    ///   `status` is not `Pending`.
    /// - [`TrellisError::ResolverCannotBeParty`] if `dispute_resolver` equals
    ///   `payer` or `payee` — the resolver must be a neutral third party.
    /// - [`TrellisError::PayerEqualsPayee`] if `payer == payee`.
    /// - [`TrellisError::MilestoneCountExceeded`] if more than
    ///   `MAX_MILESTONES` milestones are supplied.
    /// - [`TrellisError::InvalidToken`] if `token` is not a live token contract.
    /// - [`TrellisError::InvalidMilestone`] if any milestone amount is zero
    ///   or negative.
    /// - [`TrellisError::TotalAmountOverflow`] if the milestone amounts sum to
    ///   more than `i128::MAX`.
    pub fn init(
        env: Env,
        agreement_id: BytesN<32>,
        payer: Address,
        payee: Address,
        token: Address,
        milestones: Vec<Milestone>,
        dispute_resolver: Address,
    ) -> Result<(), TrellisError> {
        payer.require_auth();

        if storage::has_agreement(&env, &agreement_id) {
            return Err(TrellisError::AlreadyInitialized);
        }

        if milestones.is_empty() {
            return Err(TrellisError::EmptyMilestoneSet);
        }

        if milestones.len() > MAX_MILESTONES {
            return Err(TrellisError::MilestoneCountExceeded);
        }

        if payer == payee {
            return Err(TrellisError::PayerEqualsPayee);
        }

        if dispute_resolver == payer || dispute_resolver == payee {
            return Err(TrellisError::ResolverCannotBeParty);
        }

        // Liveness probe: the token address must be a live token contract, so
        // `symbol()` has to succeed. Both failure modes — a host trap from a
        // non-contract address and a decode failure from a contract that does
        // not return a symbol — map to `InvalidToken` rather than propagating.
        token::Client::new(&env, &token)
            .try_symbol()
            .map_err(|_| TrellisError::InvalidToken)?
            .map_err(|_| TrellisError::InvalidToken)?;

        let total_amount = validate_milestones(&milestones)?;

        let agreement = Agreement {
            agreement_id: agreement_id.clone(),
            payer: payer.clone(),
            payee: payee.clone(),
            token,
            milestones,
            dispute_resolver,
            total_amount,
            released_amounts: Map::new(&env),
        };

        storage::write_agreement(&env, &agreement_id, &agreement);
        events::agreement_created(&env, agreement_id, payer, payee);

        Ok(())
    }

    /// Lock funds for a single milestone into the contract.
    ///
    /// The payer authorises this call with `require_auth()`. The contract then
    /// pulls the tokens itself, via a single
    /// `token::Client::transfer(payer → this contract)` call — the payer's
    /// authorization *is* the authorization for that transfer, because the
    /// token contract sees the transfer as invoked by this contract on the
    /// payer's behalf and re-checks the payer's signature.
    ///
    /// No prior approval or allowance step is involved. There is no
    /// `approve` / `set_allowance` call anywhere in this crate, so callers
    /// must **not** pre-approve the escrow contract on the token contract
    /// before calling this — doing so is unnecessary, and for SAC-style
    /// tokens there is no allowance to set in the first place. The only
    /// precondition is that the payer holds at least `milestone.amount` of
    /// `agreement.token`.
    ///
    /// The milestone must be `Pending`; any other status returns
    /// [`TrellisError::InvalidStateTransition`].
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone not `Pending`.
    pub fn lock_funds(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payer.require_auth();

        // Read the milestone value, mutate it, and write it back to the same
        // slot without cloning the original entry.
        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::Pending {
            return Err(TrellisError::InvalidStateTransition);
        }

        // Mutate the milestone value and persist it back to the agreement
        // before any external calls (checks-effects-interactions pattern).
        let amount = milestone.amount;
        milestone.status = EscrowStatus::Funded;
        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        // Transfer tokens from payer → this contract.
        token::Client::new(&env, &agreement.token).transfer(
            &agreement.payer,
            &env.current_contract_address(),
            &amount,
        );

        events::funds_locked(&env, agreement_id, milestone_id, amount);

        Ok(())
    }

    /// Submit proof of work for a funded milestone.
    ///
    /// The payee authorises this call.
    ///
    /// Pass `Some(uri)` to attach delivery proof, or `None` to advance the
    /// milestone to `WorkSubmitted` without one. Empty-string proofs are
    /// normalized to `None` to ensure semantic consistency — indexers pattern
    /// match on `Some(uri)` and must never see an empty string.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone not `Funded`.
    pub fn submit_work(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
        proof_uri: Option<String>,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payee.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::Funded {
            return Err(TrellisError::InvalidStateTransition);
        }

        let proof_uri = proof_uri.filter(|s| !s.is_empty());
        milestone.status = EscrowStatus::WorkSubmitted;
        milestone.proof_uri = proof_uri.clone();
        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        events::work_submitted(&env, agreement_id, milestone_id, proof_uri);

        Ok(())
    }

    /// Approve submitted work and release funds to the payee.
    ///
    /// The payer authorises this call.
    ///
    /// Transfers whatever is still escrowed for the milestone — its full
    /// `amount` minus anything already paid out via [`Self::release_partial`]
    /// — and moves it to `Completed`. If that was the agreement's last
    /// non-terminal milestone, `agreement_completed` is emitted as well.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone not `WorkSubmitted`.
    pub fn approve_and_release(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payer.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::WorkSubmitted {
            return Err(TrellisError::InvalidStateTransition);
        }

        let amount = remaining_amount(&agreement, milestone_id, &milestone);
        milestone.status = EscrowStatus::Completed;
        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        // Transfer tokens from this contract → payee after state change.
        token::Client::new(&env, &agreement.token).transfer(
            &env.current_contract_address(),
            &agreement.payee,
            &amount,
        );

        events::funds_released(&env, agreement_id.clone(), milestone_id, amount);
        emit_if_agreement_completed(&env, &agreement_id, &agreement);

        Ok(())
    }

    /// Release part of a milestone's escrowed amount to the payee as a
    /// progress (or retainer) payment, without approving the milestone.
    ///
    /// The payer authorises this call. The milestone must be `Funded` or
    /// `WorkSubmitted` and stays in that status while any amount remains in
    /// escrow; the release that brings the remainder to zero moves it to
    /// `Completed` (emitting `agreement_completed` if it was the last
    /// non-terminal milestone).
    ///
    /// Partially-released funds are final. A later [`Self::raise_dispute`] /
    /// [`Self::resolve_dispute`] only covers what is still escrowed, and
    /// [`Self::approve_and_release`] pays out only the remainder.
    ///
    /// Returns the amount still held in escrow for the milestone afterwards.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone is not `Funded`
    ///   or `WorkSubmitted`.
    /// - [`TrellisError::InvalidReleaseAmount`] – `amount` is zero, negative,
    ///   or exceeds what is still escrowed for the milestone.
    pub fn release_partial(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
        amount: i128,
    ) -> Result<i128, TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payer.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::Funded
            && milestone.status != EscrowStatus::WorkSubmitted
        {
            return Err(TrellisError::InvalidStateTransition);
        }

        let remaining_before = remaining_amount(&agreement, milestone_id, &milestone);
        if amount <= 0 || amount > remaining_before {
            return Err(TrellisError::InvalidReleaseAmount);
        }

        // `amount <= remaining_before` bounds the new total by
        // `milestone.amount`, so neither operation can overflow.
        let released = milestone.amount - remaining_before + amount;
        let remaining = remaining_before - amount;

        agreement.released_amounts.set(milestone_id, released);
        let completed = remaining == 0;
        if completed {
            milestone.status = EscrowStatus::Completed;
            agreement.milestones.set(milestone_id, milestone);
        }
        storage::write_agreement(&env, &agreement_id, &agreement);

        // Transfer tokens from this contract → payee after state change.
        token::Client::new(&env, &agreement.token).transfer(
            &env.current_contract_address(),
            &agreement.payee,
            &amount,
        );

        events::funds_partially_released(
            &env,
            agreement_id.clone(),
            milestone_id,
            amount,
            remaining,
        );
        if completed {
            emit_if_agreement_completed(&env, &agreement_id, &agreement);
        }

        Ok(remaining)
    }

    /// Raise a dispute on a milestone that is currently `Funded` or `WorkSubmitted`.
    ///
    /// Either the payer or the payee may call this — the `caller` arg is
    /// checked against both roles so either party can autonomously trigger
    /// the dispute window.  This prevents a malicious payer from silently
    /// refusing to approve work AND refusing to raise a dispute, which would
    /// permanently lock the freelancer's funds.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::Unauthorized`] – `caller` is neither payer nor payee.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone has no funds at
    ///   stake (status is not `Funded` or `WorkSubmitted`).
    pub fn raise_dispute(
        env: Env,
        caller: Address,
        agreement_id: BytesN<32>,
        milestone_id: u32,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;

        // Check the caller is an authorised party before requiring their sig.
        if caller != agreement.payer && caller != agreement.payee {
            return Err(TrellisError::Unauthorized);
        }
        // Require the on-chain signature of whichever party is calling.
        caller.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        // Only milestones with funds at stake can be disputed.
        if milestone.status != EscrowStatus::Funded
            && milestone.status != EscrowStatus::WorkSubmitted
        {
            return Err(TrellisError::InvalidStateTransition);
        }

        let amount = remaining_amount(&agreement, milestone_id, &milestone);
        milestone.status = EscrowStatus::Disputed;
        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        events::dispute_raised(&env, agreement_id, milestone_id, caller, amount);

        Ok(())
    }

    /// Settle a disputed milestone as the designated `dispute_resolver`.
    ///
    /// Pass `payer_amount` and `payee_amount` to split the milestone's locked
    /// amount between the two parties. The two amounts must be non-negative
    /// and sum exactly to the milestone's locked amount. Passing the full
    /// amount to one side (`payer_amount = amount, payee_amount = 0` or
    /// `payer_amount = 0, payee_amount = amount`) reproduces the previous
    /// all-or-nothing behavior.
    ///
    /// # Auth
    /// `agreement.dispute_resolver.require_auth()` is the sole enforcement
    /// mechanism — the Soroban host automatically traps if the invoker's
    /// signature does not match the resolver address stored on-chain.
    /// No additional manual check is needed beyond `require_auth()`, which is
    /// why this entrypoint has no resolver-mismatch error variant: an
    /// unauthorised caller never reaches contract code at all.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone is not `Disputed`.
    /// - [`TrellisError::InvalidSplitAmounts`] – `payer_amount` or
    ///   `payee_amount` is negative, or the two do not sum to the locked amount.
    pub fn resolve_dispute(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
        payer_amount: i128,
        payee_amount: i128,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;

        // `require_auth` is the enforcement gate — the host traps if the
        // invoker is not the resolver, so a resolver-mismatch error variant
        // would be unreachable and is deliberately absent from TrellisError.
        agreement.dispute_resolver.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::Disputed {
            return Err(TrellisError::InvalidStateTransition);
        }

        let amount = milestone.amount;

        // Validate the split: both legs must be non-negative and together
        // account for exactly the locked amount. This is checked before any
        // state write or token movement so an invalid split cannot leave the
        // milestone in a half-resolved state.
        if payer_amount < 0 || payee_amount < 0 {
            return Err(TrellisError::InvalidSplitAmounts);
        }
        let total = payer_amount
            .checked_add(payee_amount)
            .ok_or(TrellisError::InvalidSplitAmounts)?;
        if total != amount {
            return Err(TrellisError::InvalidSplitAmounts);
        }

        // Status reflects the dominant outcome for indexers: a full refund to
        // the payer is `Refunded`, anything else (including a full award to
        // the payee or a genuine split) is `Completed`.
        if payee_amount == 0 {
            milestone.status = EscrowStatus::Refunded;
        } else {
            milestone.status = EscrowStatus::Completed;
        }

        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        // Settle both legs in a single transaction. Zero-amount legs are
        // skipped so we never issue a no-op transfer to the token contract.
        let token = token::Client::new(&env, &agreement.token);
        if payer_amount > 0 {
            token::Client::new(&env, &agreement.token).transfer(
                &env.current_contract_address(),
                &agreement.payer,
                &payer_amount,
            );
        }
        if payee_amount > 0 {
            token::Client::new(&env, &agreement.token).transfer(
                &env.current_contract_address(),
                &agreement.payee,
                &payee_amount,
            );
        }

        events::milestone_resolved(&env, agreement_id, milestone_id, payer_amount, payee_amount);

        Ok(())
    }

    /// Cancel a milestone that was never funded (status = `Pending`).
    ///
    /// Only the payer may withdraw a milestone proposal that has not yet had
    /// funds locked against it.  If any funds were ever locked the payer must
    /// go through the dispute flow instead.
    ///
    /// # Events
    /// Emits `("cancelled", agreement_id)` — **not** the `("resolved", …)`
    /// event used by [`Self::resolve_dispute`]. No tokens move here, so
    /// off-chain consumers must not treat a cancellation as a dispute ruling.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – `milestone_id` out of range.
    /// - [`TrellisError::InvalidStateTransition`] – milestone is not `Pending`
    ///   (i.e. funds exist or the milestone is already resolved). This is a
    ///   state machine violation, not an economic one — use the dispute flow
    ///   instead once a milestone has left `Pending`.
    pub fn cancel_unfunded_milestone(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
    ) -> Result<(), TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payer.require_auth();

        let mut milestone = agreement
            .milestones
            .get(milestone_id)
            .ok_or(TrellisError::InvalidMilestone)?;

        if milestone.status != EscrowStatus::Pending {
            // Funds exist or milestone already resolved — use dispute flow.
            return Err(TrellisError::InvalidStateTransition);
        }

        // Mark the milestone closed with no token movement required.
        let amount = milestone.amount;
        milestone.status = EscrowStatus::Refunded;
        agreement.milestones.set(milestone_id, milestone);
        storage::write_agreement(&env, &agreement_id, &agreement);

        // Emit the dedicated cancellation event rather than milestone_resolved:
        // no arbitration happened and no tokens moved, so indexers must be able
        // to tell this apart from a dispute ruling.
        events::milestone_cancelled(
            &env,
            agreement_id.clone(),
            milestone_id,
            agreement.payer.clone(),
            agreement.payer.clone(),
            amount,
        );
        emit_if_agreement_completed(&env, &agreement_id, &agreement);

        Ok(())
    }

    /// Return the full [`Agreement`] struct for the given ID.
    ///
    /// This is a read-only view — no auth is required and no state is modified.
    /// It exists primarily so the CLI `status` command can display the current
    /// agreement state (including per-milestone statuses) via
    /// `stellar contract invoke … -- get_agreement --agreement-id <hex>`.
    ///
    /// # Errors
    /// Returns [`TrellisError::AgreementNotFound`] if no agreement exists for
    /// the given `agreement_id`.
    pub fn get_agreement(env: Env, agreement_id: BytesN<32>) -> Result<Agreement, TrellisError> {
        storage::read_agreement(&env, &agreement_id)
    }

    /// Return the pre-computed total value of an agreement.
    ///
    /// Equivalent to summing `amount` over every milestone in
    /// [`Self::get_agreement`], but avoids the O(n) iteration for callers who
    /// only need the total — see [`Agreement::total_amount`].
    ///
    /// # Errors
    /// Returns [`TrellisError::AgreementNotFound`] if no agreement exists for
    /// the given `agreement_id`.
    pub fn get_total_amount(env: Env, agreement_id: BytesN<32>) -> Result<i128, TrellisError> {
        storage::read_agreement(&env, &agreement_id).map(|agreement| agreement.total_amount)
    }

    /// Fund multiple milestones in a single transaction.
    ///
    /// Iterates `milestone_ids` in order, applying the same logic as
    /// [`Self::lock_funds`] for each entry.  The entire call is atomic: if any
    /// milestone fails (out of range, wrong status), the transaction reverts and
    /// no tokens are transferred.  Individual `funds_locked` events are emitted
    /// for each successfully funded milestone so off-chain indexers retain the
    /// same per-milestone event granularity as sequential calls.
    ///
    /// The payer authorises this call once and the auth covers all transfers
    /// within the batch.
    ///
    /// # Empty input
    /// An empty `milestone_ids` is a no-op that returns `Ok(0)`. No state is
    /// written, no event is emitted and no token moves. The agreement is still
    /// read (so an unknown ID returns
    /// [`TrellisError::AgreementNotFound`]) and the payer's authorisation is
    /// still required — an empty batch is a well-formed call, not a bypass of
    /// either check.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    /// - [`TrellisError::InvalidMilestone`] – any ID in `milestone_ids` is out of range.
    /// - [`TrellisError::InvalidStateTransition`] – any milestone is not `Pending`.
    pub fn batch_lock_funds(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_ids: Vec<u32>,
    ) -> Result<u32, TrellisError> {
        let mut agreement = storage::read_agreement(&env, &agreement_id)?;
        agreement.payer.require_auth();

        // An empty batch funds nothing, so there is no state change to persist.
        // Returning here skips the `write_agreement` below, which would
        // otherwise rewrite the agreement byte-for-byte identically — a
        // redundant persistent write that costs the caller gas and extends the
        // entry's TTL while changing nothing observable.
        if milestone_ids.is_empty() {
            return Ok(0);
        }

        let token = token::Client::new(&env, &agreement.token);
        let mut funded: u32 = 0;

        for milestone_id in milestone_ids.iter() {
            let mut milestone = agreement
                .milestones
                .get(milestone_id)
                .ok_or(TrellisError::InvalidMilestone)?;

            if milestone.status != EscrowStatus::Pending {
                return Err(TrellisError::InvalidStateTransition);
            }

            let amount = milestone.amount;
            milestone.status = EscrowStatus::Funded;
            agreement.milestones.set(milestone_id, milestone);

            events::funds_locked(&env, agreement_id.clone(), milestone_id, amount);
            funded += 1;
        }

        storage::write_agreement(&env, &agreement_id, &agreement);

        for milestone_id in milestone_ids.iter() {
            let milestone = agreement
                .milestones
                .get(milestone_id)
                .ok_or(TrellisError::InvalidMilestone)?;
            token.transfer(
                &agreement.payer,
                &env.current_contract_address(),
                &milestone.amount,
            );
        }

        Ok(funded)
    }

    /// Return a single [`Milestone`] by its index within the agreement.
    ///
    /// This is a read-only view — no auth required, no state modified.  It lets
    /// callers query one milestone's current status without deserializing the
    /// full [`Agreement`] struct, which reduces ledger read cost for agreements
    /// with many milestones.
    ///
    /// Returns `None` if the agreement does not exist or `milestone_id` is out
    /// of range — both map to the same observable absence from the caller's
    /// perspective.
    ///
    /// # Return type
    /// The two `None` cases are deliberately *not* distinguished, and callers
    /// should not try to. A missing agreement and a missing milestone are both
    /// "there is no milestone at this position", and splitting them would mean
    /// either leaking agreement existence through a read-only view or adding an
    /// error variant that no caller can act on differently.
    ///
    /// Callers that need to tell them apart should use
    /// [`Self::get_agreement`] first: it returns
    /// [`TrellisError::AgreementNotFound`] for a missing ID, so
    /// `get_agreement(..).is_err()` disambiguates without any API change here.
    ///
    /// Both paths are covered separately in `test.rs`
    /// (`test_get_milestone_unknown_agreement_returns_none` for the storage miss,
    /// `test_get_milestone_invalid_id_returns_none` for the vector miss).
    pub fn get_milestone(
        env: Env,
        agreement_id: BytesN<32>,
        milestone_id: u32,
    ) -> Option<Milestone> {
        storage::read_agreement(&env, &agreement_id)
            .ok()
            .and_then(|agreement| agreement.milestones.get(milestone_id))
    }

    /// Renew the ledger TTL of an agreement without changing its state.
    ///
    /// Persistent entries are archived once their TTL runs out, which would
    /// destroy the agreement record. State-mutating entrypoints renew the TTL
    /// automatically, but an agreement that sits idle — a long delivery
    /// window, a stalled dispute — receives no writes and will eventually
    /// expire. This entrypoint exists so an external keeper service can renew
    /// it on a schedule.
    ///
    /// No auth is required: extending a TTL cannot alter agreement state and
    /// the caller pays the rent, so there is nothing to gate. Requiring a
    /// signature would only stop third-party keepers from doing useful work.
    ///
    /// # Errors
    /// - [`TrellisError::AgreementNotFound`] – unknown agreement ID.
    pub fn extend_agreement_ttl(
        env: Env,
        agreement_id: BytesN<32>,
        caller: Address,
    ) -> Result<(), TrellisError> {
        storage::extend_agreement_ttl(&env, &agreement_id)?;
        events::ttl_extended(&env, agreement_id, caller);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Amount still held in escrow for `milestone`: its configured `amount` minus
/// whatever [`TrellisContract::release_partial`] has already paid out.
fn remaining_amount(agreement: &Agreement, milestone_id: u32, milestone: &Milestone) -> i128 {
    milestone.amount - agreement.released_amounts.get(milestone_id).unwrap_or(0)
}

/// Emit `agreement_completed` if every milestone is now in a terminal state
/// (`Completed` or `Refunded`).
///
/// Called only right after an entrypoint moved one milestone into a terminal
/// state. Terminal states have no outgoing transitions, so the "all terminal"
/// condition can become true exactly once per agreement — the event therefore
/// fires at most once, on the transition that settled the last milestone.
fn emit_if_agreement_completed(env: &Env, agreement_id: &BytesN<32>, agreement: &Agreement) {
    let mut completed: u32 = 0;
    let mut refunded: u32 = 0;
    for m in agreement.milestones.iter() {
        match m.status {
            EscrowStatus::Completed => completed += 1,
            EscrowStatus::Refunded => refunded += 1,
            _ => return,
        }
    }
    events::agreement_completed(env, agreement_id.clone(), completed, refunded);
}

/// Validate incoming milestones and sum their amounts into a total.
///
/// Runs once, in `init`, before the agreement is written to storage — so an
/// invalid milestone list never consumes storage, and every later reader gets
/// the sum for free via [`Agreement::total_amount`] instead of iterating
/// `milestones` on every query.
///
/// Each milestone must be in [`EscrowStatus::Pending`]: it is the sole entry
/// point of the state machine, and every later status is reached by locking
/// funds first. Accepting a pre-advanced milestone would let its owner skip
/// `lock_funds` entirely and then release from the shared contract balance
/// tokens that were never escrowed for that milestone.
///
/// # Errors
/// Returns [`TrellisError::InvalidMilestone`] on the first milestone whose
/// `amount` is zero or negative.
/// Returns [`TrellisError::InvalidInitialMilestoneStatus`] on the first
/// milestone whose `status` is not `Pending`.
/// Returns [`TrellisError::TotalAmountOverflow`] if the sum of milestone
/// amounts exceeds i128::MAX.
fn validate_milestones(milestones: &Vec<Milestone>) -> Result<i128, TrellisError> {
    let mut total: i128 = 0;
    for m in milestones.iter() {
        if m.status != EscrowStatus::Pending {
            return Err(TrellisError::InvalidInitialMilestoneStatus);
        }
        if m.amount <= 0 {
            return Err(TrellisError::InvalidMilestone);
        }
        total = total
            .checked_add(m.amount)
            .ok_or(TrellisError::TotalAmountOverflow)?;
    }
    Ok(total)
}
