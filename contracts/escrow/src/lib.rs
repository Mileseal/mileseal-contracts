#![no_std]

//! Mileseal: milestone escrow on Soroban.
//!
//! A funder locks the full amount for every milestone of an escrow up
//! front. Each milestone pays out to one or more payees by a fixed
//! basis-point share once a payee submits work and either the funder
//! approves it, the funder stays silent past a review window, or an
//! arbiter rules on a dispute the funder raised. A milestone nobody
//! submits before its deadline refunds the funder.
//!
//! See [`MilestoneEscrow`] for the public interface.

#[cfg(test)]
extern crate std;

mod error;
mod events;
mod payout;
mod storage;
mod types;
mod validate;

pub use error::Error;
pub use types::{Escrow, Milestone, MilestoneSpec, MilestoneState, Payee};

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, BytesN, Env,
    MuxedAddress, Vec,
};

/// Milestone escrow contract. See the module docs for the overall design.
#[contract]
pub struct MilestoneEscrow;

#[contractimpl]
impl MilestoneEscrow {
    /// Creates a new escrow and locks the sum of every milestone's amount
    /// by transferring it from `funder` into the contract.
    ///
    /// Requires `funder`'s authorization. Every spec is validated before
    /// anything is written or transferred, so a rejected call locks
    /// nothing: 1..=20 milestones, each with 1..=10 payees whose shares
    /// are all > 0 and sum to exactly 10_000 basis points, a positive
    /// amount, and a deadline in the future. `review_window` must be > 0.
    ///
    /// Returns the new escrow's id.
    pub fn create(
        env: Env,
        funder: Address,
        arbiter: Address,
        token: Address,
        review_window: u64,
        specs: Vec<MilestoneSpec>,
    ) -> u64 {
        funder.require_auth();

        if review_window == 0 {
            panic_with_error!(&env, Error::InvalidInput);
        }
        validate::validate_specs(&env, &specs);

        let mut total: i128 = 0;
        let mut milestones: Vec<Milestone> = Vec::new(&env);
        for spec in specs.iter() {
            total = total
                .checked_add(spec.amount)
                .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidInput));
            milestones.push_back(Milestone {
                amount: spec.amount,
                deadline: spec.deadline,
                payees: spec.payees.clone(),
                state: MilestoneState::Pending,
                submitted_at: 0,
                proof: None,
            });
        }

        let id = storage::next_id(&env);
        let escrow = Escrow {
            funder: funder.clone(),
            arbiter,
            token: token.clone(),
            review_window,
            milestones,
        };
        storage::write_escrow(&env, id, &escrow);

        let client = TokenClient::new(&env, &token);
        client.transfer(
            &funder,
            MuxedAddress::from(env.current_contract_address()),
            &total,
        );

        events::Created {
            escrow_id: id,
            funder,
            token,
            total_amount: total,
            milestone_count: specs.len(),
        }
        .publish(&env);

        id
    }

    /// Submits a deliverable for one milestone. Requires `payee`'s
    /// authorization, and `payee` must hold a share of that milestone.
    /// The milestone must be `Pending` and its deadline must not have
    /// passed. Moves the milestone to `Submitted`.
    pub fn submit(env: Env, id: u64, index: u32, payee: Address, proof: BytesN<32>) {
        payee.require_auth();

        let mut escrow = storage::read_escrow(&env, id);
        let mut milestone = get_milestone(&env, &escrow, index);

        if milestone.state != MilestoneState::Pending {
            panic_with_error!(&env, Error::WrongState);
        }
        if env.ledger().timestamp() > milestone.deadline {
            panic_with_error!(&env, Error::DeadlinePassed);
        }
        if !validate::is_payee(&milestone.payees, &payee) {
            panic_with_error!(&env, Error::Unauthorized);
        }

        milestone.submitted_at = env.ledger().timestamp();
        milestone.proof = Some(proof.clone());
        milestone.state = MilestoneState::Submitted;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Submitted {
            escrow_id: id,
            index,
            payee,
            proof,
        }
        .publish(&env);
    }

    /// Approves a submitted milestone, paying every payee their share.
    /// Requires the escrow's funder's authorization. The milestone must
    /// be `Submitted`. Moves the milestone to `Released`.
    pub fn approve(env: Env, id: u64, index: u32) {
        let mut escrow = storage::read_escrow(&env, id);
        escrow.funder.require_auth();

        let mut milestone = get_milestone(&env, &escrow, index);
        if milestone.state != MilestoneState::Submitted {
            panic_with_error!(&env, Error::WrongState);
        }

        payout::distribute(&env, &escrow.token, &milestone.payees, milestone.amount);

        milestone.state = MilestoneState::Released;
        let amount = milestone.amount;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Released {
            escrow_id: id,
            index,
            amount,
            claimed: false,
        }
        .publish(&env);
    }

    /// Lets any payee of a submitted milestone claim payment once the
    /// funder has gone silent past the review window. Requires `payee`'s
    /// authorization, and `payee` must hold a share of that milestone.
    /// Pays every payee their share, the same as [`Self::approve`].
    /// Moves the milestone to `Released`.
    pub fn claim_unreviewed(env: Env, id: u64, index: u32, payee: Address) {
        payee.require_auth();

        let mut escrow = storage::read_escrow(&env, id);
        let mut milestone = get_milestone(&env, &escrow, index);

        if milestone.state != MilestoneState::Submitted {
            panic_with_error!(&env, Error::WrongState);
        }
        if !validate::is_payee(&milestone.payees, &payee) {
            panic_with_error!(&env, Error::Unauthorized);
        }
        let review_ends = milestone
            .submitted_at
            .checked_add(escrow.review_window)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidInput));
        if env.ledger().timestamp() < review_ends {
            panic_with_error!(&env, Error::ReviewWindow);
        }

        payout::distribute(&env, &escrow.token, &milestone.payees, milestone.amount);

        milestone.state = MilestoneState::Released;
        let amount = milestone.amount;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Released {
            escrow_id: id,
            index,
            amount,
            claimed: true,
        }
        .publish(&env);
    }

    /// Disputes a submitted milestone before its review window elapses.
    /// Requires the escrow's funder's authorization. Moves the milestone
    /// to `Disputed`, where only the arbiter's [`Self::resolve`] can move
    /// it further; there is no timeout if the arbiter never rules.
    pub fn dispute(env: Env, id: u64, index: u32) {
        let mut escrow = storage::read_escrow(&env, id);
        escrow.funder.require_auth();

        let mut milestone = get_milestone(&env, &escrow, index);
        if milestone.state != MilestoneState::Submitted {
            panic_with_error!(&env, Error::WrongState);
        }
        let review_ends = milestone
            .submitted_at
            .checked_add(escrow.review_window)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidInput));
        if env.ledger().timestamp() >= review_ends {
            panic_with_error!(&env, Error::ReviewWindow);
        }

        milestone.state = MilestoneState::Disputed;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Disputed {
            escrow_id: id,
            index,
        }
        .publish(&env);
    }

    /// Rules on a disputed milestone. Requires the escrow's arbiter's
    /// authorization. `payee_bps` (0..=10_000) is the share of the
    /// milestone's amount paid to the payees, split by their own shares;
    /// the rest returns to the funder. Moves the milestone to `Resolved`.
    pub fn resolve(env: Env, id: u64, index: u32, payee_bps: u32) {
        let mut escrow = storage::read_escrow(&env, id);
        escrow.arbiter.require_auth();

        let mut milestone = get_milestone(&env, &escrow, index);
        if milestone.state != MilestoneState::Disputed {
            panic_with_error!(&env, Error::WrongState);
        }
        if payee_bps > validate::BPS_DENOM {
            panic_with_error!(&env, Error::InvalidInput);
        }

        let payee_amount = payout::bps_of(&env, milestone.amount, payee_bps);
        let funder_amount = milestone
            .amount
            .checked_sub(payee_amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidInput));

        payout::distribute(&env, &escrow.token, &milestone.payees, payee_amount);
        if funder_amount > 0 {
            let client = TokenClient::new(&env, &escrow.token);
            client.transfer(
                &env.current_contract_address(),
                MuxedAddress::from(escrow.funder.clone()),
                &funder_amount,
            );
        }

        milestone.state = MilestoneState::Resolved;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Resolved {
            escrow_id: id,
            index,
            payee_bps,
            payee_amount,
            funder_amount,
        }
        .publish(&env);
    }

    /// Refunds an expired, never-submitted milestone to the funder.
    /// Callable by anyone once the milestone's deadline has strictly
    /// passed. The milestone must still be `Pending`. Moves the milestone
    /// to `Refunded`.
    pub fn refund_expired(env: Env, id: u64, index: u32) {
        let mut escrow = storage::read_escrow(&env, id);
        let mut milestone = get_milestone(&env, &escrow, index);

        if milestone.state != MilestoneState::Pending {
            panic_with_error!(&env, Error::WrongState);
        }
        if env.ledger().timestamp() <= milestone.deadline {
            panic_with_error!(&env, Error::NotExpired);
        }

        let client = TokenClient::new(&env, &escrow.token);
        client.transfer(
            &env.current_contract_address(),
            MuxedAddress::from(escrow.funder.clone()),
            &milestone.amount,
        );

        milestone.state = MilestoneState::Refunded;
        let amount = milestone.amount;
        escrow.milestones.set(index, milestone);
        storage::write_escrow(&env, id, &escrow);

        events::Refunded {
            escrow_id: id,
            index,
            amount,
        }
        .publish(&env);
    }

    /// Returns the full state of an escrow, or panics with
    /// [`Error::NotFound`] if `id` is unknown.
    pub fn get(env: Env, id: u64) -> Escrow {
        storage::read_escrow(&env, id)
    }

    /// Returns how many escrows have been created.
    pub fn count(env: Env) -> u64 {
        storage::count(&env)
    }
}

/// Fetches milestone `index` out of `escrow`, panicking with
/// [`Error::BadIndex`] if it is out of range.
fn get_milestone(env: &Env, escrow: &Escrow, index: u32) -> Milestone {
    escrow
        .milestones
        .get(index)
        .unwrap_or_else(|| panic_with_error!(env, Error::BadIndex))
}

#[cfg(test)]
mod test;
