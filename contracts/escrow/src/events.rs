use soroban_sdk::{contractevent, Address, BytesN};

/// Emitted when [`crate::MilestoneEscrow::create`] locks funds for a new escrow.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Created {
    #[topic]
    pub escrow_id: u64,
    pub funder: Address,
    pub token: Address,
    pub total_amount: i128,
    pub milestone_count: u32,
}

/// Emitted when a payee submits a deliverable for a milestone.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submitted {
    #[topic]
    pub escrow_id: u64,
    pub index: u32,
    pub payee: Address,
    pub proof: BytesN<32>,
}

/// Emitted when a milestone is paid out, by approval or by an unreviewed claim.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Released {
    #[topic]
    pub escrow_id: u64,
    pub index: u32,
    pub amount: i128,
    pub claimed: bool,
}

/// Emitted when the funder disputes a submitted milestone.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disputed {
    #[topic]
    pub escrow_id: u64,
    pub index: u32,
}

/// Emitted when the arbiter rules on a disputed milestone.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    #[topic]
    pub escrow_id: u64,
    pub index: u32,
    pub payee_bps: u32,
    pub payee_amount: i128,
    pub funder_amount: i128,
}

/// Emitted when an expired, unsubmitted milestone is refunded to the funder.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    #[topic]
    pub escrow_id: u64,
    pub index: u32,
    pub amount: i128,
}
