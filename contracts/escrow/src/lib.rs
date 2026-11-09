#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct MilestoneEscrow;

#[contractimpl]
impl MilestoneEscrow {
    pub fn count(_env: Env) -> u64 {
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn scaffold_compiles_and_links() {
        let env = Env::default();
        let contract_id = env.register(MilestoneEscrow, ());
        let client = MilestoneEscrowClient::new(&env, &contract_id);
        assert_eq!(client.count(), 0);
    }
}
