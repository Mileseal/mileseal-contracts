use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env};

use crate::error::Error;
use crate::types::Escrow;

/// Ledger count used for TTL math, assuming ~5 seconds per ledger.
const LEDGERS_PER_DAY: u32 = 17_280;
/// Every TTL extension reaches 30 days of remaining life.
const BUMP_TO: u32 = LEDGERS_PER_DAY * 30;
/// An entry is extended once its remaining TTL drops below 7 days.
const THRESHOLD: u32 = LEDGERS_PER_DAY * 7;

#[contracttype]
enum DataKey {
    Escrow(u64),
}

/// Allocates the next escrow id. Ids start at 0 and increment by 1;
/// [`count`] always reports how many ids have been allocated so far.
pub fn next_id(env: &Env) -> u64 {
    let key = symbol_short!("COUNT");
    let id: u64 = env.storage().instance().get(&key).unwrap_or(0);
    env.storage().instance().set(&key, &(id + 1));
    env.storage().instance().extend_ttl(THRESHOLD, BUMP_TO);
    id
}

/// Returns how many escrows have been created.
pub fn count(env: &Env) -> u64 {
    let key = symbol_short!("COUNT");
    env.storage().instance().get(&key).unwrap_or(0)
}

/// Loads an escrow by id, extending its storage TTL, or panics with
/// [`Error::NotFound`].
pub fn read_escrow(env: &Env, id: u64) -> Escrow {
    let key = DataKey::Escrow(id);
    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotFound));
    env.storage()
        .persistent()
        .extend_ttl(&key, THRESHOLD, BUMP_TO);
    escrow
}

/// Writes an escrow back to storage and extends its TTL.
pub fn write_escrow(env: &Env, id: u64, escrow: &Escrow) {
    let key = DataKey::Escrow(id);
    env.storage().persistent().set(&key, escrow);
    env.storage()
        .persistent()
        .extend_ttl(&key, THRESHOLD, BUMP_TO);
}
