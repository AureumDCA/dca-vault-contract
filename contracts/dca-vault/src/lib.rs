#![no_std]
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, token, vec, Address, ConversionError,
    Env, EnvBase, IntoVal, MapObject, Symbol, TryFromVal, TryIntoVal, Val, Vec,
};

/// Ledger close time is ~5s, so a day is roughly this many ledgers.
const LEDGERS_PER_DAY: u32 = 17280;

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schedule {
    pub frequency: Frequency,
    pub amount_per_execution: i128,
    pub target_asset: Address,
    pub last_execution_ledger: u32,
    pub next_execution_ledger: u32,
    pub pool_address: Address,
    pub min_amount_out_bps: u32,
}

// Not #[contracttype]: soroban-sdk 26.1.0's contracttype derive generates an
// XDR/ScVal conversion (active whenever the `testutils` feature is enabled,
// e.g. under `cargo test`) that doesn't support a struct field typed
// `Option<OtherContractTypeStruct>`. Vault embeds `Option<Schedule>`, so it
// hits that bug. The impls below replicate only the runtime Env/Val
// conversion the macro would otherwise generate, which has no such
// restriction and is all a contract actually needs at execution time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vault {
    pub owner: Address,
    pub balance: i128,
    pub schedule: Option<Schedule>,
    pub paused: bool,
}

impl TryFromVal<Env, Val> for Vault {
    type Error = ConversionError;
    fn try_from_val(env: &Env, val: &Val) -> Result<Self, ConversionError> {
        const KEYS: [&'static str; 4] = ["balance", "owner", "paused", "schedule"];
        let mut vals: [Val; 4] = [Val::VOID.to_val(); 4];
        let map: MapObject = val.try_into().map_err(|_| ConversionError)?;
        env.map_unpack_to_slice(map, &KEYS, &mut vals)
            .map_err(|_| ConversionError)?;
        Ok(Self {
            balance: vals[0].try_into_val(env).map_err(|_| ConversionError)?,
            owner: vals[1].try_into_val(env).map_err(|_| ConversionError)?,
            paused: vals[2].try_into_val(env).map_err(|_| ConversionError)?,
            schedule: vals[3].try_into_val(env).map_err(|_| ConversionError)?,
        })
    }
}

impl TryFromVal<Env, Vault> for Val {
    type Error = ConversionError;
    fn try_from_val(env: &Env, val: &Vault) -> Result<Self, ConversionError> {
        const KEYS: [&'static str; 4] = ["balance", "owner", "paused", "schedule"];
        let vals: [Val; 4] = [
            (&val.balance).try_into_val(env).map_err(|_| ConversionError)?,
            (&val.owner).try_into_val(env).map_err(|_| ConversionError)?,
            (&val.paused).try_into_val(env).map_err(|_| ConversionError)?,
            (&val.schedule).try_into_val(env).map_err(|_| ConversionError)?,
        ];
        Ok(env
            .map_new_from_slices(&KEYS, &vals)
            .map_err(|_| ConversionError)?
            .into())
    }
}

impl TryFromVal<Env, &Vault> for Val {
    type Error = ConversionError;
    fn try_from_val(env: &Env, val: &&Vault) -> Result<Self, ConversionError> {
        <_ as TryFromVal<Env, Vault>>::try_from_val(env, *val)
    }
}

#[contracttype]
enum DataKey {
    Token,
    Vault(Address),
}

/// Emitted by [`DcaVaultContract::execute_swap`] on every successful swap.
/// Topics: static `"swap"` + `owner` address (indexed for backend queries).
/// Data: a Map with `amount_in`, `amount_out` (i128), and `pool_address`.
#[contractevent(topics = ["swap"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapExecuted {
    #[topic]
    pub owner: Address,
    pub amount_in: i128,
    pub amount_out: i128,
    pub pool_address: Address,
}

/// Emitted by [`DcaVaultContract::create_schedule`] whenever a schedule is
/// attached to a vault (including replacing an existing one). Lets the
/// backend indexer discover a vault the moment it's scheduled, instead of
/// waiting for its first `SwapExecuted` event.
/// Topics: static `"schedule_created"` + `owner` address.
/// Data: a Map with `frequency`, `amount_per_execution` (i128),
/// `target_asset`, and `pool_address`.
#[contractevent(topics = ["schedule_created"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleCreated {
    #[topic]
    pub owner: Address,
    pub frequency: Frequency,
    pub amount_per_execution: i128,
    pub target_asset: Address,
    pub pool_address: Address,
}

/// Internal abstraction over "a contract that can execute a swap for us".
/// Soroban contracts have no host function for the classic Stellar SDEX (no
/// XDR opcode/host call reaches it), so scheduled swaps must go through a
/// contract-to-contract call into an AMM/liquidity-pool-style contract
/// instead. This trait lets `execute_swap` stay agnostic to which pool
/// implementation backs a given schedule.
pub trait SwapPool {
    fn swap(
        env: &Env,
        pool_address: &Address,
        token_in: &Address,
        token_out: &Address,
        amount_in: i128,
        min_amount_out: i128,
    ) -> i128;
}

/// Calls into a pool contract expected to expose:
/// `swap(to: Address, token_in: Address, token_out: Address, amount_in: i128,
/// min_amount_out: i128) -> i128`.
///
/// The vault pushes `amount_in` of `token_in` to the pool itself first (a
/// direct, self-authorizing transfer — the vault is the contract currently
/// executing, so no signature is needed), then invokes `swap` with
/// `to = env.current_contract_address()`. The pool is expected to send
/// `amount_out` of `token_out` back to `to` using its own (also
/// self-authorizing) transfer. Every transfer in this flow is a direct call
/// made by the contract that owns the funds being moved, so no
/// `authorize_as_current_contract` plumbing for deeper calls is needed; if
/// the pool's `min_amount_out` check fails and it panics, the whole
/// transaction (including the earlier push) reverts atomically.
///
/// soroban-examples' `liquidity_pool` contract was read for reference, but
/// its actual interface (`swap(to, buy_a: bool, out, in_max)`, fixed
/// two-token A/B pools, exact-output amounts, no return value) doesn't match
/// the generic token_in/token_out/exact-input shape this vault needs, so
/// `GenericPoolAdapter` targets the simpler generic ABI above instead —
/// future adapters can be added for pools with other interfaces.
pub struct GenericPoolAdapter;

impl SwapPool for GenericPoolAdapter {
    fn swap(
        env: &Env,
        pool_address: &Address,
        token_in: &Address,
        token_out: &Address,
        amount_in: i128,
        min_amount_out: i128,
    ) -> i128 {
        let to = env.current_contract_address();
        token::TokenClient::new(env, token_in).transfer(&to, pool_address, &amount_in);

        let args: Vec<Val> = vec![
            env,
            to.into_val(env),
            token_in.into_val(env),
            token_out.into_val(env),
            amount_in.into_val(env),
            min_amount_out.into_val(env),
        ];
        env.invoke_contract::<i128>(pool_address, &Symbol::new(env, "swap"), args)
    }
}

#[contract]
pub struct DcaVaultContract;

#[contractimpl]
impl DcaVaultContract {
    /// Sets the token contract (the XLM Stellar Asset Contract on a real
    /// network) that deposits/withdrawals move. Must be called once before
    /// any other function.
    pub fn initialize(env: Env, token: Address) {
        if env.storage().instance().has(&DataKey::Token) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn deposit(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_client = Self::token_client(&env);
        token_client.transfer(&owner, &env.current_contract_address(), &amount);

        let mut vault = Self::load_or_create_vault(&env, &owner);
        vault.balance += amount;
        env.storage().persistent().set(&DataKey::Vault(owner), &vault);
    }

    pub fn withdraw(env: Env, owner: Address, amount: i128) {
        owner.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let mut vault = Self::get_vault(env.clone(), owner.clone());
        if amount > vault.balance {
            panic!("withdrawal amount exceeds balance");
        }
        vault.balance -= amount;
        env.storage()
            .persistent()
            .set(&DataKey::Vault(owner.clone()), &vault);

        let token_client = Self::token_client(&env);
        token_client.transfer(&env.current_contract_address(), &owner, &amount);
    }

    pub fn create_schedule(
        env: Env,
        owner: Address,
        frequency: Frequency,
        amount_per_execution: i128,
        target_asset: Address,
        pool_address: Address,
        min_amount_out_bps: u32,
    ) {
        owner.require_auth();
        if amount_per_execution <= 0 {
            panic!("amount_per_execution must be positive");
        }
        if min_amount_out_bps > 10_000 {
            panic!("min_amount_out_bps must be <= 10000");
        }

        let mut vault = Self::load_or_create_vault(&env, &owner);
        let current_ledger = env.ledger().sequence();
        vault.schedule = Some(Schedule {
            frequency,
            amount_per_execution,
            target_asset: target_asset.clone(),
            last_execution_ledger: current_ledger,
            next_execution_ledger: current_ledger + Self::ledgers_for(frequency),
            pool_address: pool_address.clone(),
            min_amount_out_bps,
        });
        env.storage()
            .persistent()
            .set(&DataKey::Vault(owner.clone()), &vault);

        ScheduleCreated {
            owner,
            frequency,
            amount_per_execution,
            target_asset,
            pool_address,
        }
        .publish(&env);
    }

    pub fn pause_schedule(env: Env, owner: Address) {
        owner.require_auth();
        let mut vault = Self::get_vault(env.clone(), owner.clone());
        vault.paused = true;
        env.storage().persistent().set(&DataKey::Vault(owner), &vault);
    }

    pub fn resume_schedule(env: Env, owner: Address) {
        owner.require_auth();
        let mut vault = Self::get_vault(env.clone(), owner.clone());
        vault.paused = false;
        env.storage().persistent().set(&DataKey::Vault(owner), &vault);
    }

    pub fn get_vault(env: Env, owner: Address) -> Vault {
        env.storage()
            .persistent()
            .get(&DataKey::Vault(owner))
            .expect("vault does not exist")
    }

    /// Executes one due, unpaused schedule's swap. Permissionless: anyone
    /// (e.g. a keeper bot) may call this to trigger a vault's scheduled
    /// swap — no auth is required from `owner` or the caller, since the
    /// funds being moved already belong to this contract (deposited
    /// earlier) and every transfer below is a direct, self-authorizing
    /// call. The schedule's own due/paused/balance checks are what gate
    /// execution, not caller identity.
    pub fn execute_swap(env: Env, owner: Address) -> i128 {
        let mut vault = Self::get_vault(env.clone(), owner.clone());
        if vault.paused {
            panic!("schedule is paused");
        }
        let mut schedule = vault.schedule.clone().expect("no schedule configured");

        let current_ledger = env.ledger().sequence();
        if current_ledger < schedule.next_execution_ledger {
            panic!("schedule is not yet due");
        }
        if vault.balance < schedule.amount_per_execution {
            panic!("insufficient balance for scheduled swap");
        }

        let token_in = Self::token_address(&env);
        let token_out = schedule.target_asset.clone();
        let pool_address = schedule.pool_address.clone();
        let amount_in = schedule.amount_per_execution;

        // TODO: replace this naive 1:1 expected-output assumption with a
        // real price/impact calculation (e.g. via an oracle or the pool's
        // own quote function) once one is available.
        let min_amount_out = amount_in.saturating_mul(schedule.min_amount_out_bps as i128) / 10_000;

        let amount_out = GenericPoolAdapter::swap(
            &env,
            &pool_address,
            &token_in,
            &token_out,
            amount_in,
            min_amount_out,
        );

        vault.balance -= amount_in;
        schedule.last_execution_ledger = current_ledger;
        schedule.next_execution_ledger = current_ledger + Self::ledgers_for(schedule.frequency);
        vault.schedule = Some(schedule);
        env.storage()
            .persistent()
            .set(&DataKey::Vault(owner.clone()), &vault);

        SwapExecuted {
            owner: owner.clone(),
            amount_in,
            amount_out,
            pool_address,
        }
        .publish(&env);

        amount_out
    }

    fn load_or_create_vault(env: &Env, owner: &Address) -> Vault {
        env.storage()
            .persistent()
            .get(&DataKey::Vault(owner.clone()))
            .unwrap_or(Vault {
                owner: owner.clone(),
                balance: 0,
                schedule: None,
                paused: false,
            })
    }

    fn token_address(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("contract not initialized")
    }

    fn token_client(env: &Env) -> token::TokenClient<'_> {
        token::TokenClient::new(env, &Self::token_address(env))
    }

    fn ledgers_for(frequency: Frequency) -> u32 {
        match frequency {
            Frequency::Daily => LEDGERS_PER_DAY,
            Frequency::Weekly => LEDGERS_PER_DAY * 7,
            Frequency::Monthly => LEDGERS_PER_DAY * 30,
        }
    }
}

#[cfg(test)]
mod test;
