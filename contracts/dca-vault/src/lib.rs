#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, ConversionError, Env, EnvBase,
    MapObject, TryFromVal, TryIntoVal, Val,
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
    ) {
        owner.require_auth();
        if amount_per_execution <= 0 {
            panic!("amount_per_execution must be positive");
        }

        let mut vault = Self::load_or_create_vault(&env, &owner);
        let current_ledger = env.ledger().sequence();
        vault.schedule = Some(Schedule {
            frequency,
            amount_per_execution,
            target_asset,
            last_execution_ledger: current_ledger,
            next_execution_ledger: current_ledger + Self::ledgers_for(frequency),
        });
        env.storage().persistent().set(&DataKey::Vault(owner), &vault);

        // TODO: swap execution adapter (next feature)
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

    fn token_client(env: &Env) -> token::TokenClient<'_> {
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("contract not initialized");
        token::TokenClient::new(env, &token_address)
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
