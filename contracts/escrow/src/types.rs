use soroban_sdk::{contracttype, Address, BytesN, Vec};

/// A single payout recipient of a milestone, with a basis-point share.
/// 10_000 basis points equal 100%.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payee {
    pub addr: Address,
    pub bps: u32,
}

/// The caller-supplied description of one milestone, used only at
/// [`crate::MilestoneEscrow::create`] time to build a [`Milestone`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneSpec {
    pub amount: i128,
    pub deadline: u64,
    pub payees: Vec<Payee>,
}

/// The lifecycle state of a single milestone.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneState {
    /// Waiting for a payee to submit work before the deadline.
    Pending,
    /// A payee submitted work; waiting on the funder within the review window.
    Submitted,
    /// Paid out to the payees, either by approval or by an unreviewed claim.
    Released,
    /// The funder disputed the submission within the review window.
    Disputed,
    /// The arbiter ruled on a dispute and funds were split and paid out.
    Resolved,
    /// The deadline passed with no submission; funds returned to the funder.
    Refunded,
}

/// One milestone of an escrow, holding both its terms and its current state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub amount: i128,
    pub deadline: u64,
    pub payees: Vec<Payee>,
    pub state: MilestoneState,
    pub submitted_at: u64,
    pub proof: Option<BytesN<32>>,
}

/// A funder's escrow: a fixed set of milestones funded up front, with an
/// arbiter chosen at creation time to rule on disputes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub funder: Address,
    pub arbiter: Address,
    pub token: Address,
    pub review_window: u64,
    pub milestones: Vec<Milestone>,
}
