use soroban_sdk::{panic_with_error, Env, Vec};

use crate::error::Error;
use crate::types::{MilestoneSpec, Payee};

/// Maximum number of milestones a single escrow may hold.
pub const MAX_MILESTONES: u32 = 20;
/// Maximum number of payees a single milestone may split a payout between.
pub const MAX_PAYEES: u32 = 10;
/// Basis points representing 100%.
pub const BPS_DENOM: u32 = 10_000;

/// Validates every milestone spec supplied to `create`. Panics with
/// [`Error::InvalidInput`] on the first problem found, before anything is
/// written to storage or transferred.
pub fn validate_specs(env: &Env, specs: &Vec<MilestoneSpec>) {
    if specs.is_empty() || specs.len() > MAX_MILESTONES {
        panic_with_error!(env, Error::InvalidInput);
    }
    let now = env.ledger().timestamp();
    for spec in specs.iter() {
        if spec.amount <= 0 {
            panic_with_error!(env, Error::InvalidInput);
        }
        if spec.deadline <= now {
            panic_with_error!(env, Error::InvalidInput);
        }
        validate_payees(env, &spec.payees);
    }
}

/// Validates one milestone's payee list: 1..=10 payees, every share > 0,
/// and shares summing to exactly [`BPS_DENOM`].
pub fn validate_payees(env: &Env, payees: &Vec<Payee>) {
    if payees.is_empty() || payees.len() > MAX_PAYEES {
        panic_with_error!(env, Error::InvalidInput);
    }
    let mut sum: u32 = 0;
    for payee in payees.iter() {
        if payee.bps == 0 {
            panic_with_error!(env, Error::InvalidInput);
        }
        sum = match sum.checked_add(payee.bps) {
            Some(sum) => sum,
            None => panic_with_error!(env, Error::InvalidInput),
        };
    }
    if sum != BPS_DENOM {
        panic_with_error!(env, Error::InvalidInput);
    }
}

/// Returns `true` if `addr` holds a share in `payees`.
pub fn is_payee(payees: &Vec<Payee>, addr: &soroban_sdk::Address) -> bool {
    payees.iter().any(|payee| &payee.addr == addr)
}
