use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

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

    client.create_schedule(&owner, &Frequency::Weekly, &50, &target_asset);

    let vault = client.get_vault(&owner);
    let schedule = vault.schedule.expect("schedule should be set");
    assert_eq!(schedule.frequency, Frequency::Weekly);
    assert_eq!(schedule.amount_per_execution, 50);
    assert_eq!(schedule.target_asset, target_asset);
}

#[test]
fn pause_and_resume_schedule_toggle_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _token_address) = setup(&env);
    let client = DcaVaultContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let target_asset = Address::generate(&env);
    client.create_schedule(&owner, &Frequency::Daily, &10, &target_asset);

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
