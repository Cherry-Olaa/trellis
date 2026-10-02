use soroban_sdk::contracterror;

/// Canonical error type for the Trellis Protocol contract.
///
/// `#[contracterror]` serialises each variant's `u32` discriminant into the
/// XDR `ScError` envelope returned to the invoker, making error codes part of the
/// public on-chain ABI.
///
/// # Stability rule
/// Discriminant values are **permanent** from the first mainnet deployment
/// onwards: never renumber an existing variant, only append new ones at the
/// end.  While the protocol is pre-release (no mainnet deployment yet) the
/// numbering is still being compacted — variant `6` (`NotDisputeResolver`)
/// was removed because no codepath could ever return it, and `NoFundsToRefund`
/// was renumbered from `7` to `6` to close the gap. `NoFundsToRefund` (then
/// discriminant `6`) was itself later removed for the same reason: no
/// codepath ever returned it. Discriminant `6` is left vacant rather than
/// reused, per the append-only rule above. SDT consumers pinned to the old
/// numbering must regenerate their bindings.
///
/// # Exhaustiveness
/// `#[non_exhaustive]` is what makes the append-only rule above enforceable by
/// the compiler rather than by convention. Without it, any downstream `match`
/// over `TrellisError` that enumerates the current variants is accepted today
/// and becomes a hard compile error the next time a variant is appended —
/// turning a documented stability guarantee into a breaking change for
/// consumers. With it, downstream matches are required to carry a wildcard arm
/// from the start, so appending a variant stays non-breaking.
///
/// This is the same treatment [`crate::types::EscrowStatus`] already has, and
/// for the same reason: both are append-only enums in the public ABI. No `match`
/// inside this crate is exhaustive over `TrellisError` (every site either
/// constructs an error or compares against one), so the attribute costs
/// nothing here.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
#[non_exhaustive]
pub enum TrellisError {
    /// The contract or agreement has already been initialised.
    /// Prevents duplicate `create_agreement` calls for the same ID.
    AlreadyInitialized = 1,

    /// The caller is not permitted to perform this action.
    /// Covers payer-only, payee-only, and resolver-only guards.
    Unauthorized = 2,

    /// No agreement exists in ledger storage for the supplied ID.
    AgreementNotFound = 3,

    /// A milestone is invalid — e.g. zero amount or out-of-range index.
    InvalidMilestone = 4,

    /// The requested operation is illegal given the current `EscrowStatus`.
    /// e.g. attempting to release funds before work has been submitted.
    InvalidStateTransition = 5,

    // Discriminant 6 vacant — formerly `NoFundsToRefund`, removed as
    // dead code: no codepath ever returned it. `cancel_unfunded_milestone`// only ever runs while a milestone still holds no funds, so calling it
    // on a milestone that has left `Pending` is a state machine violation
    // ([`TrellisError::InvalidStateTransition`]), not a distinct economic
    // one. Left vacant rather than reused, per the append-only rule above.
    /// `init` was called with an empty `milestones` vector. An agreement with
    /// no milestones can never transition through any state, permanently
    /// wasting the storage it occupies.
    EmptyMilestoneSet = 7,

    /// `init` was called with a `dispute_resolver` equal to `payer` or
    /// `payee`. The resolver must be a neutral third party; allowing it to
    /// coincide with either party would let that party unilaterally decide
    /// its own disputes.
    ResolverCannotBeParty = 8,

    /// Total milestone amount exceeds i128::MAX during summation.
    /// A crafted agreement with sufficiently large milestones would cause
    /// silent integer wraparound, corrupting the total_amount field.
    TotalAmountOverflow = 9,

    /// Token address is invalid or not a valid token contract.
    /// The liveness probe (symbol() call) failed to verify the address
    /// represents an active, functional token contract.
    InvalidToken = 10,

    /// `init` was called with a milestone whose `status` is not
    /// [`EscrowStatus::Pending`].
    ///
    /// Every agreement sharing a token draws from one pooled contract
    /// balance, so a milestone created in a pre-advanced state is a claim on
    /// funds that were never escrowed for it. A `WorkSubmitted` milestone
    /// could go straight to `approve_and_release` and a `Disputed` one to
    /// `resolve_dispute`, either of which transfers tokens out of the pool
    /// to the payee or back to the payer without anything having been
    /// locked. `Pending` is the only valid initial state: it is the sole
    /// entry point of the state machine, and every later transition is
    /// reached by funding the milestone first.
    ///
    /// Appended as discriminant `11` per the stability rule above; it is a
    /// distinct economic condition from [`TrellisError::InvalidMilestone`]
    /// (which covers amounts and indices) and deserves its own code so an
    /// integrator can tell "you sent a bad amount" from "you tried to
    /// pre-advance a milestone".
    InvalidInitialMilestoneStatus = 11,

    /// The payer/payee split supplied to `resolve_dispute` does not sum
    /// to the milestone's locked amount. A split resolution must account
    /// for every unit of escrowed funds exactly once; any other total either
    /// leaves funds stranded in the contract or attempts to pay out more
    /// than was locked.
    InvalidSplitAmounts = 12,

    /// `init` was called with more than `MAX_MILESTONES` milestones.
    /// An oversized milestone list would make every later read, write and
    /// status transition cost unbounded gas, so it is rejected up front.
    MilestoneCountExceeded = 13,

    /// `init` was called with `payer == payee`. A single address cannot both
    /// fund the escrow and be paid out of it, which would let one party
    /// unilaterally release its own funds.
    PayerEqualsPayee = 14,

    /// `release_partial` was called with an `amount` that is zero, negative,
    /// or larger than what is still escrowed for that milestone. The release
    /// must be strictly positive and can never exceed the milestone's
    /// remaining balance, so the sum paid out for a milestone is capped by
    /// what was locked for it.
    InvalidReleaseAmount = 15,
}
