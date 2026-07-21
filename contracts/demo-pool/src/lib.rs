#![no_std]
//! This is a minimal demo AMM pool built solely to provide a real, callable
//! testnet counterparty for dca-vault-contract's execute_swap function and
//! GenericPoolAdapter. It is intentionally simple (fixed 1:1 rate, no price
//! curve) and is not intended as a production AMM.
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
enum DataKey {
    Admin,
    TokenA,
    TokenB,
}

#[contract]
pub struct DemoPool;

#[contractimpl]
impl DemoPool {
    /// One-time setup: records the admin and the pool's two supported
    /// tokens. Callable once.
    pub fn initialize(env: Env, admin: Address, token_a: Address, token_b: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenA, &token_a);
        env.storage().instance().set(&DataKey::TokenB, &token_b);
    }

    /// Admin-authorized: pulls `amount` of `token` from `from` into the
    /// pool's own balance, to be used as swap liquidity. `from` is
    /// authorized directly (rather than checking against the stored admin)
    /// so any funded account can top up the pool for testing.
    pub fn deposit_liquidity(env: Env, from: Address, token: Address, amount: i128) {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        token::TokenClient::new(&env, &token).transfer(
            &from,
            &env.current_contract_address(),
            &amount,
        );
    }

    /// Executes a swap at a fixed 1:1 rate. NOT production logic: a real AMM
    /// would price this off a curve (e.g. constant product) reflecting pool
    /// reserves; this demo pool exists only to give dca-vault-contract's
    /// `execute_swap` a real, callable testnet counterparty implementing the
    /// exact `GenericPoolAdapter` ABI, so genuine end-to-end swaps can be
    /// verified on testnet. Follows the push-then-call convention already
    /// established by `GenericPoolAdapter`: the caller (the vault) is
    /// expected to have already transferred `amount_in` of `token_in` to
    /// this pool before invoking `swap`, so this function's job is only to
    /// pay out `token_out`. No auth is required, matching the permissionless
    /// design `execute_swap` relies on.
    pub fn swap(
        env: Env,
        to: Address,
        _token_in: Address,
        token_out: Address,
        amount_in: i128,
        min_amount_out: i128,
    ) -> i128 {
        let amount_out = amount_in;
        if amount_out < min_amount_out {
            panic!("slippage: amount_out below min_amount_out");
        }

        let token_out_client = token::TokenClient::new(&env, &token_out);
        let pool_balance = token_out_client.balance(&env.current_contract_address());
        if pool_balance < amount_out {
            panic!("insufficient pool liquidity");
        }

        token_out_client.transfer(&env.current_contract_address(), &to, &amount_out);
        amount_out
    }

    /// Read-only: the pool's current balance of `token`, i.e. its available
    /// liquidity for that asset.
    pub fn get_balance(env: Env, token: Address) -> i128 {
        token::TokenClient::new(&env, &token).balance(&env.current_contract_address())
    }
}

#[cfg(test)]
mod test;
