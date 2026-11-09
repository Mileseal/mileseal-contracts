use soroban_sdk::{panic_with_error, token::TokenClient, Address, Env, MuxedAddress, Vec};

use crate::error::Error;
use crate::types::Payee;
use crate::validate::BPS_DENOM;

/// Transfers `amount` out of the contract's own balance, split among
/// `payees` by their basis-point shares. Each payee but the last gets
/// `floor(amount * bps / BPS_DENOM)`; the last payee gets whatever
/// remains, so the transfers always sum to exactly `amount`.
///
/// Does nothing if `amount` is 0.
pub fn distribute(env: &Env, token: &Address, payees: &Vec<Payee>, amount: i128) {
    if amount == 0 {
        return;
    }
    let client = TokenClient::new(env, token);
    let from = env.current_contract_address();
    let n = payees.len();
    let mut distributed: i128 = 0;
    for i in 0..n {
        let payee = payees.get_unchecked(i);
        let share = if i == n - 1 {
            amount
                .checked_sub(distributed)
                .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput))
        } else {
            let share = bps_of(env, amount, payee.bps);
            distributed = distributed
                .checked_add(share)
                .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput));
            share
        };
        if share > 0 {
            client.transfer(&from, MuxedAddress::from(payee.addr.clone()), &share);
        }
    }
}

/// Returns `floor(amount * bps / BPS_DENOM)`, using checked arithmetic.
/// `amount` is always non-negative in this contract, so integer division
/// truncating toward zero is equivalent to flooring.
pub fn bps_of(env: &Env, amount: i128, bps: u32) -> i128 {
    amount
        .checked_mul(bps as i128)
        .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput))
        / (BPS_DENOM as i128)
}
