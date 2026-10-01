use soroban_sdk::{contracttype, Address, BytesN, Map, String, Vec};

// ---------------------------------------------------------------------------
// EscrowStatus — lifecycle state machine for an escrow agreement / milestone
// ---------------------------------------------------------------------------
#[contracttype]
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    /// Agreement created but no funds deposited yet.
    Pending,
    /// Payer has deposited the agreed token amount into the contract.
    Funded,
    /// Payee has submitted proof of work and is awaiting payer approval.
    WorkSubmitted,
    /// Payer approved the milestone; funds released to payee.
    Completed,
    /// Either party raised a dispute; awaiting resolver arbitration.
    Disputed,
    /// Funds returned to the payer after a dispute ruling in the payer's
    /// favour. Written only by `resolve_dispute` — real tokens moved.
    Refunded,
    /// Milestone cancelled while still `Pending`, before any funds were ever
    /// locked. Written only by `cancel_unfunded_milestone`, which moves no
    /// tokens — kept distinct from [`EscrowStatus::Refunded`] so readers of
    /// on-chain state can tell a never-funded cancellation apart from a
    /// dispute refund without replaying the event log.
    Cancelled,
}

// ---------------------------------------------------------------------------
// Milestone — a single deliverable within an Agreement
//
// `Eq`/`PartialEq` are derived so a `Milestone` read back from the contract can
// be compared to an expected value with a single `assert_eq!` instead of a
// hand-written field-by-field loop. Deriving them costs nothing at runtime —
// `#[contracttype]` serialisation is unchanged — and it means a new field added
// here is automatically covered by every existing equality assertion, rather
// than silently left out of one that only happens to pluck the fields it knew
// about when it was written.
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    /// Token amount (in the smallest denomination) locked for this milestone.
    ///
    /// Must be strictly positive — `init` rejects zero or negative amounts
    /// with [`crate::errors::TrellisError::InvalidMilestone`], since a
    /// zero-value milestone creates a noise transaction with no economic
    /// effect and a negative amount is not a meaningful escrow value.
    pub amount: i128,
    /// Current lifecycle state of this milestone.
    pub status: EscrowStatus,
    /// Optional URI linking to delivery proof (e.g. GitHub PR, Figma file).
    ///
    /// `None` means no proof has been submitted yet. This is the only
    /// representation of "no proof" — an empty `Some("")` is not a sentinel
    /// and callers should not construct one.
    pub proof_uri: Option<String>,
    /// Optional split of this milestone's locked amount between payer and
    /// payee, set when a dispute is resolved with a partial outcome.
    ///
    /// `None` means no split has been recorded (all-or-nothing resolution or
    /// no dispute). When `Some`, `payer_amount + payee_amount` must equal
    /// `amount` exactly.
    pub split: Option<MilestoneSplit>,
}

// ---------------------------------------------------------------------------
// Agreement — top-level escrow record stored on-chain
//
// See the note on [`Milestone`] for why `Eq`/`PartialEq` are derived: it lets a
// whole agreement read back from `get_agreement` be asserted with one
// `assert_eq!`, so no field can be added here without every existing equality
// assertion noticing.
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneSplit {
    /// Portion of the milestone amount awarded back to the payer.
    pub payer_amount: i128,
    /// Portion of the milestone amount awarded to the payee.
    pub payee_amount: i128,
}

// ---------------------------------------------------------------------------
// Agreement — top-level escrow record stored on-chain
//
// See the note on [`Milestone`] for why `Eq`/`PartialEq` are derived: it lets a
// whole agreement read back from `get_agreement` be asserted with one
// `assert_eq!`, so no field can be added here without every existing equality
// assertion noticing.
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agreement {
    /// Globally unique identifier for this agreement (32-byte hash).
    pub agreement_id: BytesN<32>,
    /// The party funding the escrow (client / buyer).
    pub payer: Address,
    /// The party delivering work and receiving funds (contractor / seller).
    pub payee: Address,
    /// SAC or custom token contract used for payments.
    pub token: Address,
    /// Ordered list of milestones that make up this agreement.
    ///
    /// Must be non-empty — `init` rejects a `milestones` vector with zero
    /// entries, since such an agreement could never transition through any
    /// state.
    pub milestones: Vec<Milestone>,
    /// Trusted third-party address authorised to resolve disputes.
    ///
    /// Must be distinct from both `payer` and `payee` — `init` rejects a
    /// resolver equal to either party, since that would let one side
    /// unilaterally decide its own disputes.
    pub dispute_resolver: Address,
    /// Sum of every milestone's `amount`, pre-computed once in `init`.
    ///
    /// Lets off-chain readers (indexers, the CLI, the frontend) get the
    /// agreement's total value from a single field instead of iterating
    /// `milestones` on every read. Fixed for the lifetime of the agreement —
    /// there is no entrypoint that adds, removes, or resizes milestones after
    /// `init`, so it never needs recomputation.
    pub total_amount: i128,
    /// Cumulative amount already paid out to the payee per milestone via
    /// partial releases, keyed by milestone index.
    ///
    /// Empty at `init`; only `release_partial` inserts entries. A milestone
    /// with no entry has released nothing. The amount still held in escrow
    /// for a milestone is always `milestone.amount - released_amounts[id]`,
    /// and every later payout (`approve_and_release`, `resolve_dispute`)
    /// moves only that remainder, so the sum paid out for a milestone can
    /// never exceed what was locked for it.
    ///
    /// Stored inside the agreement record rather than under its own ledger
    /// key so it shares the agreement's TTL: a separately-archived counter
    /// would silently read back as zero and allow a double payout.
    pub released_amounts: Map<u32, i128>,
}
