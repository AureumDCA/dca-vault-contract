use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{vec as svec, Env, IntoVal};

fn setup(env: &Env) -> (Address, Address) {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token_address = sac.address();

    let contract_id = env.register(DcaVaultContract, ());
    let client = DcaVaultContractClient::new(env, &contract_id);
    client.initialize(&token_address);

    (contract_id, token_address)
}

fn fund(env: &Env, token_address: &Address, to: &Address, amount: i128) {
    let token_admin = token::StellarAssetClient::new(env, token_address);
    token_admin.mint(to, &amount);
}

/// Minimal mock AMM/pool contract implementing the `GenericPoolAdapter`
/// target ABI: fixed 1:1 swap rate, assumes the caller already pushed
/// `amount_in` of `token_in` to this pool before calling (per the
/// push-then-call convention documented on `GenericPoolAdapter`), and pays
/// out `token_out` from its own pre-funded liquidity.
#[contract]
struct MockPool;

#[contractimpl]
impl MockPool {
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
        token::TokenClient::new(&env, &token_out).transfer(
            &env.current_contract_address(),
            &to,
            &amount_out,
        );
        amount_out
    }
}

/// Sets up a vault contract plus a funded mock pool and target asset, ready
/// for `create_schedule` + `execute_swap` tests.
fn setup_swap(env: &Env) -> (Address, Address, Address, Address) {
    let (contract_id, xlm) = setup(env);

    let target_admin = Address::generate(env);
    let target_sac = env.register_stellar_asset_contract_v2(target_admin);
    let target_asset = target_sac.address();

    let pool_id = env.register(MockPool, ());
    let target_token_admin = token::StellarAssetClient::new(env, &target_asset);
    target_token_admin.mint(&pool_id, &1_000_000);

    (contract_id, xlm, target_asset, pool_id)
}

fn make_due(env: &Env, ledger: u32) {
    env.ledger().with_mut(|li| li.sequence_number = ledger);
}

#[test]
fn deposit_increases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &token_address, &owner, 1_000);

    client.deposit(&owner, &400);

    let vault = client.get_vault(&owner);
    assert_eq!(vault.balance, 400);
}

#[test]
fn get_vault_with_no_schedule_returns_none() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &token_address, &owner, 1_000);

    client.deposit(&owner, &400);

    let vault = client.get_vault(&owner);
    assert_eq!(vault.owner, owner);
    assert_eq!(vault.balance, 400);
    assert_eq!(vault.schedule, None);
}

#[test]
fn withdraw_decreases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &token_address, &owner, 1_000);

    client.deposit(&owner, &500);
    client.withdraw(&owner, &200);

    let vault = client.get_vault(&owner);
    assert_eq!(vault.balance, 300);
}

#[test]
#[should_panic(expected = "withdrawal amount exceeds balance")]
fn withdraw_more_than_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &token_address, &owner, 1_000);

    client.deposit(&owner, &100);
    client.withdraw(&owner, &200);
}

#[test]
fn create_schedule_attaches_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let target_asset = Address::generate(&env);
    let pool_address = Address::generate(&env);

    client.create_schedule(
        &owner,
        &Frequency::Weekly,
        &50,
        &target_asset,
        &pool_address,
        &9_500,
    );

    let vault = client.get_vault(&owner);
    let schedule = vault.schedule.expect("schedule should be set");
    assert_eq!(schedule.frequency, Frequency::Weekly);
    assert_eq!(schedule.amount_per_execution, 50);
    assert_eq!(schedule.target_asset, target_asset);
    assert_eq!(schedule.pool_address, pool_address);
    assert_eq!(schedule.min_amount_out_bps, 9_500);
}

#[test]
fn pause_and_resume_schedule_toggle_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let target_asset = Address::generate(&env);
    let pool_address = Address::generate(&env);
    client.create_schedule(
        &owner,
        &Frequency::Daily,
        &10,
        &target_asset,
        &pool_address,
        &9_500,
    );

    client.pause_schedule(&owner);
    assert!(client.get_vault(&owner).paused);

    client.resume_schedule(&owner);
    assert!(!client.get_vault(&owner).paused);
}

#[test]
#[should_panic(expected = "vault does not exist")]
fn get_vault_on_nonexistent_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    client.get_vault(&owner);
}

#[test]
fn execute_swap_succeeds_when_due() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, xlm, target_asset, pool_id) = setup_swap(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &xlm, &owner, 1_000);
    client.deposit(&owner, &500);
    client.create_schedule(&owner, &Frequency::Daily, &100, &target_asset, &pool_id, &9_000);

    let due_ledger = client
        .get_vault(&owner)
        .schedule
        .unwrap()
        .next_execution_ledger;
    make_due(&env, due_ledger);

    let amount_out = client.execute_swap(&owner);
    assert_eq!(amount_out, 100);

    // events().all() only returns events from the *last* contract
    // invocation, so it must be captured before any further client calls.
    // It also includes the underlying token transfer events nested inside
    // that invocation, so filter down to events the vault contract itself
    // published.
    let events = env.events().all().filter_by_contract(&contract_id);

    let vault = client.get_vault(&owner);
    assert_eq!(vault.balance, 400);
    let schedule = vault.schedule.unwrap();
    assert_eq!(schedule.last_execution_ledger, due_ledger);
    assert_eq!(schedule.next_execution_ledger, due_ledger + LEDGERS_PER_DAY);

    assert_eq!(
        events,
        svec![
            &env,
            (
                contract_id.clone(),
                svec![&env, symbol_short!("swap").into_val(&env), owner.into_val(&env)],
                (100i128, 100i128, pool_id).into_val(&env),
            )
        ]
    );
}

#[test]
#[should_panic(expected = "schedule is not yet due")]
fn execute_swap_panics_when_not_due() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, xlm, target_asset, pool_id) = setup_swap(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &xlm, &owner, 1_000);
    client.deposit(&owner, &500);
    client.create_schedule(&owner, &Frequency::Daily, &100, &target_asset, &pool_id, &9_000);

    client.execute_swap(&owner);
}

#[test]
#[should_panic(expected = "schedule is paused")]
fn execute_swap_panics_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, xlm, target_asset, pool_id) = setup_swap(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &xlm, &owner, 1_000);
    client.deposit(&owner, &500);
    client.create_schedule(&owner, &Frequency::Daily, &100, &target_asset, &pool_id, &9_000);

    let due_ledger = client
        .get_vault(&owner)
        .schedule
        .unwrap()
        .next_execution_ledger;
    make_due(&env, due_ledger);
    client.pause_schedule(&owner);

    client.execute_swap(&owner);
}

#[test]
#[should_panic(expected = "insufficient balance for scheduled swap")]
fn execute_swap_panics_when_balance_insufficient() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, xlm, target_asset, pool_id) = setup_swap(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    fund(&env, &xlm, &owner, 1_000);
    client.deposit(&owner, &50);
    client.create_schedule(&owner, &Frequency::Daily, &100, &target_asset, &pool_id, &9_000);

    let due_ledger = client
        .get_vault(&owner)
        .schedule
        .unwrap()
        .next_execution_ledger;
    make_due(&env, due_ledger);

    client.execute_swap(&owner);
}

#[test]
fn execute_swap_is_callable_by_non_owner() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, xlm, target_asset, pool_id) = setup_swap(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let keeper = Address::generate(&env);
    assert_ne!(owner, keeper);

    fund(&env, &xlm, &owner, 1_000);
    client.deposit(&owner, &500);
    client.create_schedule(&owner, &Frequency::Daily, &100, &target_asset, &pool_id, &9_000);

    let due_ledger = client
        .get_vault(&owner)
        .schedule
        .unwrap()
        .next_execution_ledger;
    make_due(&env, due_ledger);

    // Keeper triggers the swap for owner's vault; owner never signs this
    // call. mock_all_auths() would silently satisfy any require_auth() call
    // that *did* fire, so the real assertion that this is permissionless is
    // dropping all mocked auths right before the call: if execute_swap (or
    // anything it calls) required owner's or keeper's auth, this would panic.
    env.set_auths(&[]);
    let amount_out = client.execute_swap(&owner);
    assert_eq!(amount_out, 100);
    assert_eq!(client.get_vault(&owner).balance, 400);
}
