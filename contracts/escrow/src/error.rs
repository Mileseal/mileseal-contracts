use soroban_sdk::contracterror;

/// Errors returned by the MilestoneEscrow contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// No escrow exists for the given id.
    NotFound = 1,
    /// The milestone index is out of range for the escrow.
    BadIndex = 2,
    /// Input failed validation (bad amounts, shares, deadlines, or sizes).
    InvalidInput = 3,
    /// The caller is not entitled to perform this action.
    Unauthorized = 4,
    /// The milestone is not in the state this action requires.
    WrongState = 5,
    /// The milestone's submission deadline has already passed.
    DeadlinePassed = 6,
    /// The action is not permitted relative to the review window.
    ReviewWindow = 7,
    /// The milestone has not yet expired.
    NotExpired = 8,
}
