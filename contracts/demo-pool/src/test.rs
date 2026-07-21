use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

fn setup(env: &Env) -> (Address, Address, Address, Address) {
    let admin = Address::generate(env);
    let token_a_admin = Address::generate(env);
    let token_b_admin = Address::generate(env);
    let token_a = env.register_stellar_asset_contract_v2(token_a_admin).address();
    let token_b = env.register_stellar_asset_contract_v2(token_b_admin).address();

    let contract_id = env.register(DemoPool, ());
    let client = DemoPoolClient::new(env, &contract_id);
    client.initialize(&admin, &token_a, &token_b);

    (contract_id, admin, token_a, token_b)
}

fn fund(env: &Env, token: &Address, to: &Address, amount: i128) {
    let token_admin = token::StellarAssetClient::new(env, token);
    token_admin.mint(to, &amount);
}

#[test]
fn deposit_liquidity_increases_pool_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _admin, token_a, _token_b) = setup(&env);
    let client = DemoPoolClient::new(&env, &contract_id);

    let funder = Address::generate(&env);
    fund(&env, &token_a, &funder, 1_000);

    client.deposit_liquidity(&funder, &token_a, &400);

    assert_eq!(client.get_balance(&token_a), 400);
}

#[test]
fn swap_succeeds_and_transfers_at_1_to_1_rate() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, admin, token_a, token_b) = setup(&env);
    let client = DemoPoolClient::new(&env, &contract_id);

    // Fund the pool with token_b liquidity (what the swap will pay out).
    fund(&env, &token_b, &admin, 1_000);
    client.deposit_liquidity(&admin, &token_b, &1_000);

    // Simulate the vault's push-then-call convention: the caller transfers
    // token_a into the pool itself before invoking swap.
    let recipient = Address::generate(&env);
    fund(&env, &token_a, &contract_id, 100);

    let amount_out = client.swap(&recipient, &token_a, &token_b, &100, &95);

    assert_eq!(amount_out, 100);
    assert_eq!(
        token::TokenClient::new(&env, &token_b).balance(&recipient),
        100
    );
    assert_eq!(client.get_balance(&token_b), 900);
}

#[test]
#[should_panic(expected = "slippage: amount_out below min_amount_out")]
fn swap_panics_when_min_amount_out_exceeds_1_to_1_output() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, admin, token_a, token_b) = setup(&env);
    let client = DemoPoolClient::new(&env, &contract_id);

    fund(&env, &token_b, &admin, 1_000);
    client.deposit_liquidity(&admin, &token_b, &1_000);

    let recipient = Address::generate(&env);
    fund(&env, &token_a, &contract_id, 100);

    // 1:1 rate means amount_out is 100; requiring 101 must panic.
    client.swap(&recipient, &token_a, &token_b, &100, &101);
}

#[test]
#[should_panic(expected = "insufficient pool liquidity")]
fn swap_panics_when_pool_liquidity_insufficient() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, admin, token_a, token_b) = setup(&env);
    let client = DemoPoolClient::new(&env, &contract_id);

    // Fund the pool with far less token_b than the swap requests.
    fund(&env, &token_b, &admin, 10);
    client.deposit_liquidity(&admin, &token_b, &10);

    let recipient = Address::generate(&env);
    fund(&env, &token_a, &contract_id, 100);

    client.swap(&recipient, &token_a, &token_b, &100, &95);
}

#[test]
fn get_balance_reflects_deposits_and_swaps() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, admin, token_a, token_b) = setup(&env);
    let client = DemoPoolClient::new(&env, &contract_id);

    assert_eq!(client.get_balance(&token_a), 0);
    assert_eq!(client.get_balance(&token_b), 0);

    fund(&env, &token_b, &admin, 500);
    client.deposit_liquidity(&admin, &token_b, &500);
    assert_eq!(client.get_balance(&token_b), 500);

    let recipient = Address::generate(&env);
    fund(&env, &token_a, &contract_id, 200);
    client.swap(&recipient, &token_a, &token_b, &200, &200);

    // Paying out 200 of token_b leaves 300; the pushed 200 of token_a
    // landed directly in the pool via the push-then-call convention.
    assert_eq!(client.get_balance(&token_b), 300);
    assert_eq!(client.get_balance(&token_a), 200);
}
