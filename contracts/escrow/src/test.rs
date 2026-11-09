#![cfg(test)]

use crate::{Error, MilestoneEscrow, MilestoneEscrowClient, MilestoneSpec, MilestoneState, Payee};
use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, BytesN, Env, IntoVal, Vec,
};

const DAY: u64 = 86_400;

struct Fixture<'a> {
    env: Env,
    client: MilestoneEscrowClient<'a>,
    contract_id: Address,
    funder: Address,
    arbiter: Address,
    token: Address,
    token_client: token::TokenClient<'a>,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let arbiter = Address::generate(&env);

    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token = sac.address();
    token::StellarAssetClient::new(&env, &token).mint(&funder, &1_000_000_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);
    let token_client = token::TokenClient::new(&env, &token);

    Fixture {
        env,
        client,
        contract_id,
        funder,
        arbiter,
        token,
        token_client,
    }
}

fn one_payee_spec(env: &Env, payee: &Address, amount: i128, deadline: u64) -> MilestoneSpec {
    let mut payees = Vec::new(env);
    payees.push_back(Payee {
        addr: payee.clone(),
        bps: 10_000,
    });
    MilestoneSpec {
        amount,
        deadline,
        payees,
    }
}

fn two_payee_spec(
    env: &Env,
    a: &Address,
    a_bps: u32,
    b: &Address,
    b_bps: u32,
    amount: i128,
    deadline: u64,
) -> MilestoneSpec {
    let mut payees = Vec::new(env);
    payees.push_back(Payee {
        addr: a.clone(),
        bps: a_bps,
    });
    payees.push_back(Payee {
        addr: b.clone(),
        bps: b_bps,
    });
    MilestoneSpec {
        amount,
        deadline,
        payees,
    }
}

fn proof(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

fn contract_err(e: Error) -> soroban_sdk::Error {
    soroban_sdk::Error::from_contract_error(e as u32)
}

// --- create -----------------------------------------------------------

#[test]
fn create_locks_the_exact_total() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(
        &f.env,
        [
            one_payee_spec(&f.env, &payee, 100, deadline),
            one_payee_spec(&f.env, &payee, 250, deadline),
        ],
    );

    let before = f.token_client.balance(&f.funder);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    let after = f.token_client.balance(&f.funder);

    assert_eq!(before - after, 350);
    assert_eq!(f.token_client.balance(&f.contract_id), 350);
    let escrow = f.client.get(&id);
    assert_eq!(escrow.milestones.len(), 2);
}

#[test]
fn ids_increment() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 10, deadline)]);

    assert_eq!(f.client.count(), 0);
    let id0 = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    let id1 = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    let id2 = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    assert_eq!((id0, id1, id2), (0, 1, 2));
    assert_eq!(f.client.count(), 3);
}

#[test]
fn invalid_creation_input_locks_nothing() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let now = f.env.ledger().timestamp();
    let deadline = now + DAY;
    let before = f.token_client.balance(&f.funder);

    // Empty milestone list.
    let empty: Vec<MilestoneSpec> = Vec::new(&f.env);
    assert!(f
        .client
        .try_create(&f.funder, &f.arbiter, &f.token, &1000, &empty)
        .is_err());

    // Bad bps sum.
    let bad_bps = Vec::from_array(
        &f.env,
        [two_payee_spec(
            &f.env,
            &payee,
            6_000,
            &Address::generate(&f.env),
            3_000,
            100,
            deadline,
        )],
    );
    assert!(f
        .client
        .try_create(&f.funder, &f.arbiter, &f.token, &1000, &bad_bps)
        .is_err());

    // Deadline not in the future.
    let past = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, now)]);
    assert!(f
        .client
        .try_create(&f.funder, &f.arbiter, &f.token, &1000, &past)
        .is_err());

    // Zero amount.
    let zero_amount = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 0, deadline)]);
    assert!(f
        .client
        .try_create(&f.funder, &f.arbiter, &f.token, &1000, &zero_amount)
        .is_err());

    // Zero review window.
    let ok_spec = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    assert!(f
        .client
        .try_create(&f.funder, &f.arbiter, &f.token, &0, &ok_spec)
        .is_err());

    let after = f.token_client.balance(&f.funder);
    assert_eq!(before, after);
    assert_eq!(f.client.count(), 0);
}

// --- submit -------------------------------------------------------------

#[test]
fn only_a_payee_can_submit() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let stranger = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    let result = f.client.try_submit(&id, &0, &stranger, &proof(&f.env, 1));
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::Unauthorized)
    );
}

#[test]
fn no_submit_after_the_deadline() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    f.env.ledger().set_timestamp(deadline + 1);
    let result = f.client.try_submit(&id, &0, &payee, &proof(&f.env, 1));
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::DeadlinePassed)
    );
}

#[test]
fn submit_at_the_deadline_still_works() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    f.env.ledger().set_timestamp(deadline);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    let escrow = f.client.get(&id);
    assert_eq!(
        escrow.milestones.get_unchecked(0).state,
        MilestoneState::Submitted
    );
}

#[test]
fn bad_index() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    let result = f.client.try_submit(&id, &5, &payee, &proof(&f.env, 1));
    assert_eq!(result.unwrap_err().unwrap(), contract_err(Error::BadIndex));
}

#[test]
fn unknown_escrow() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let result = f.client.try_submit(&999, &0, &payee, &proof(&f.env, 1));
    assert_eq!(result.unwrap_err().unwrap(), contract_err(Error::NotFound));
}

// --- approve -------------------------------------------------------------

#[test]
fn approve_pays_by_share() {
    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(
        &f.env,
        [two_payee_spec(
            &f.env, &a, 7_000, &b, 3_000, 1_000, deadline,
        )],
    );
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &a, &proof(&f.env, 1));
    f.client.approve(&id, &0);

    assert_eq!(f.token_client.balance(&a), 700);
    assert_eq!(f.token_client.balance(&b), 300);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
    let escrow = f.client.get(&id);
    assert_eq!(
        escrow.milestones.get_unchecked(0).state,
        MilestoneState::Released
    );
}

#[test]
fn rounding_dust_goes_to_the_last_payee_and_contract_ends_at_zero() {
    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    // 100 split 33.33%/66.67% (3_333 / 6_667 bps) over amount=100:
    // a gets floor(100*3_333/10_000)=33, b (last) gets 100-33=67.
    let specs = Vec::from_array(
        &f.env,
        [two_payee_spec(&f.env, &a, 3_333, &b, 6_667, 100, deadline)],
    );
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &a, &proof(&f.env, 1));
    f.client.approve(&id, &0);

    assert_eq!(f.token_client.balance(&a), 33);
    assert_eq!(f.token_client.balance(&b), 67);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
}

#[test]
fn no_double_approve() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    f.client.approve(&id, &0);

    let result = f.client.try_approve(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::WrongState)
    );
}

#[test]
fn approve_before_submission_fails() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    let result = f.client.try_approve(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::WrongState)
    );
}

#[test]
fn approve_with_the_wrong_signer_fails() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let stranger = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));

    let args: Vec<soroban_sdk::Val> = (id, 0u32).into_val(&f.env);
    let result = f
        .client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &f.contract_id,
                fn_name: "approve",
                args,
                sub_invokes: &[],
            },
        }])
        .try_approve(&id, &0);
    assert!(result.is_err());
}

// --- claim_unreviewed -----------------------------------------------------

#[test]
fn claim_unreviewed_fails_inside_the_window() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));

    let result = f.client.try_claim_unreviewed(&id, &0, &payee);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::ReviewWindow)
    );
}

#[test]
fn claim_unreviewed_works_at_the_boundary() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    let submitted_at = f.env.ledger().timestamp();

    f.env.ledger().set_timestamp(submitted_at + 1000);
    f.client.claim_unreviewed(&id, &0, &payee);

    assert_eq!(f.token_client.balance(&payee), 100);
    let escrow = f.client.get(&id);
    assert_eq!(
        escrow.milestones.get_unchecked(0).state,
        MilestoneState::Released
    );
}

// --- dispute / resolve ---------------------------------------------------

#[test]
fn dispute_then_60_40_resolve_with_exact_balances() {
    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(
        &f.env,
        [two_payee_spec(
            &f.env, &a, 5_000, &b, 5_000, 1_000, deadline,
        )],
    );
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &a, &proof(&f.env, 1));

    let funder_before = f.token_client.balance(&f.funder);
    f.client.dispute(&id, &0);
    f.client.resolve(&id, &0, &6_000);

    // payees get 60% of 1000 = 600, split 50/50 -> 300 each.
    assert_eq!(f.token_client.balance(&a), 300);
    assert_eq!(f.token_client.balance(&b), 300);
    // funder gets the remaining 40% = 400 back.
    assert_eq!(f.token_client.balance(&f.funder) - funder_before, 400);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);

    let escrow = f.client.get(&id);
    assert_eq!(
        escrow.milestones.get_unchecked(0).state,
        MilestoneState::Resolved
    );
}

#[test]
fn resolve_fully_for_either_side() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;

    // Fully to the payee.
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    f.client.dispute(&id, &0);
    f.client.resolve(&id, &0, &10_000);
    assert_eq!(f.token_client.balance(&payee), 100);

    // Fully back to the funder. Measure right after the lock (not before
    // it), since the create() call itself already deducts the 100 that
    // resolve() is expected to hand back.
    let id2 = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    let funder_after_lock = f.token_client.balance(&f.funder);
    f.client.submit(&id2, &0, &payee, &proof(&f.env, 2));
    f.client.dispute(&id2, &0);
    f.client.resolve(&id2, &0, &0);
    assert_eq!(f.token_client.balance(&f.funder) - funder_after_lock, 100);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
    assert_eq!(f.token_client.balance(&payee), 100); // unchanged from before
}

#[test]
fn resolve_rejects_over_100_percent_and_the_wrong_state() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    // Wrong state: not disputed yet.
    let result = f.client.try_resolve(&id, &0, &5_000);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::WrongState)
    );

    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    f.client.dispute(&id, &0);

    // Over 100%.
    let result = f.client.try_resolve(&id, &0, &10_001);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::InvalidInput)
    );
}

#[test]
fn dispute_fails_after_the_window() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));
    let submitted_at = f.env.ledger().timestamp();

    f.env.ledger().set_timestamp(submitted_at + 1000);
    let result = f.client.try_dispute(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::ReviewWindow)
    );
}

// --- refund_expired -------------------------------------------------------

#[test]
fn refund_expired_fails_at_the_deadline_and_works_after_then_cannot_repeat() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);

    f.env.ledger().set_timestamp(deadline);
    let result = f.client.try_refund_expired(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::NotExpired)
    );

    let funder_before = f.token_client.balance(&f.funder);
    f.env.ledger().set_timestamp(deadline + 1);
    f.client.refund_expired(&id, &0);
    assert_eq!(f.token_client.balance(&f.funder) - funder_before, 100);

    let result = f.client.try_refund_expired(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::WrongState)
    );
}

#[test]
fn cannot_refund_a_submitted_milestone() {
    let f = setup();
    let payee = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let specs = Vec::from_array(&f.env, [one_payee_spec(&f.env, &payee, 100, deadline)]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &payee, &proof(&f.env, 1));

    f.env.ledger().set_timestamp(deadline + 1);
    let result = f.client.try_refund_expired(&id, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        contract_err(Error::WrongState)
    );
}

// --- SDK fixtures for the TypeScript encoder/decoder ----------------------

#[test]
#[ignore]
fn dump_sdk_fixtures() {
    use soroban_sdk::xdr::ToXdr;
    use std::io::Write;

    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    let deadline = f.env.ledger().timestamp() + DAY;
    let spec = two_payee_spec(&f.env, &a, 7_000, &b, 3_000, 1_000, deadline);
    let specs = Vec::from_array(&f.env, [spec.clone()]);
    let id = f
        .client
        .create(&f.funder, &f.arbiter, &f.token, &1000, &specs);
    f.client.submit(&id, &0, &a, &proof(&f.env, 9));
    let escrow = f.client.get(&id);

    let escrow_xdr = escrow.to_xdr(&f.env);
    let spec_xdr = spec.to_xdr(&f.env);

    let to_hex = |bytes: soroban_sdk::Bytes| -> std::string::String {
        let mut v = std::vec![0u8; bytes.len() as usize];
        bytes.copy_into_slice(&mut v);
        v.iter().map(|b| std::format!("{:02x}", b)).collect()
    };

    let json = std::format!(
        "{{\"funder\":\"{}\",\"arbiter\":\"{}\",\"token\":\"{}\",\"a\":\"{}\",\"b\":\"{}\",\"escrowXdr\":\"{}\",\"specXdr\":\"{}\"}}",
        f.funder.to_string(),
        f.arbiter.to_string(),
        f.token.to_string(),
        a.to_string(),
        b.to_string(),
        to_hex(escrow_xdr),
        to_hex(spec_xdr),
    );
    // `Address::to_string()` returns a `soroban_sdk::String` (it implements
    // `Display`), which `format!` writes out fine without extra conversion.

    let out_path =
        std::env::var("MILESEAL_FIXTURES_OUT").expect("MILESEAL_FIXTURES_OUT must be set");
    let mut file = std::fs::File::create(out_path).expect("failed to create fixtures file");
    file.write_all(json.as_bytes())
        .expect("failed to write fixtures file");
}
