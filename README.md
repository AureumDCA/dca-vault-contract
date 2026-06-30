# DCA Vault Contract

A Soroban smart contract on Stellar implementing a trustless Dollar-Cost
Averaging (DCA) vault: users deposit XLM into a personal on-chain vault,
configure a recurring swap schedule, and anyone can permissionlessly trigger
execution of a due, unpaused schedule's swap — without a custodian.

## The DCA Vault concept

Dollar-cost averaging means buying a fixed amount of an asset on a fixed
schedule (e.g. weekly) instead of all at once, smoothing out entry price over
time. This contract gives every depositor their own on-chain `Vault`:

- **Balance** — XLM the user has deposited but not yet committed to a
  scheduled swap.
- **Schedule** — an optional, user-configured recurring purchase: how often
  (`Daily` / `Weekly` / `Monthly`), how much per execution, which asset to
  buy, which pool contract to swap through (`pool_address`), and a slippage
  tolerance (`min_amount_out_bps`).
- **Paused flag** — lets the owner halt scheduled execution without
  withdrawing funds or deleting the schedule.

Anyone (not just the owner — e.g. a keeper bot) can permissionlessly trigger
a due, unpaused schedule's swap via `execute_swap`; the schedule's own
due/paused/balance checks gate execution, not caller identity.

## Functions

| Function | Description |
| --- | --- |
| `initialize(token)` | One-time setup: records the token contract (XLM's Stellar Asset Contract) that deposits/withdrawals move. |
| `deposit(owner, amount)` | Owner-authorized. Transfers `amount` XLM from `owner` into the vault, increasing its balance. |
| `withdraw(owner, amount)` | Owner-authorized. Decreases the vault balance and transfers `amount` XLM back to `owner`. Panics if `amount` exceeds the balance. |
| `create_schedule(owner, frequency, amount_per_execution, target_asset, pool_address, min_amount_out_bps)` | Owner-authorized. Creates or replaces the owner's recurring swap schedule. |
| `pause_schedule(owner)` | Owner-authorized. Halts scheduled execution without clearing the schedule. |
| `resume_schedule(owner)` | Owner-authorized. Re-enables a paused schedule. |
| `get_vault(owner)` | Read-only. Returns the owner's `Vault`, or panics if none exists. |
| `execute_swap(owner) -> i128` | Permissionless. Executes one due, unpaused schedule's swap through its configured pool, updates the vault's balance/ledgers, emits a `swap` event, and returns the amount received. Panics if the schedule is missing, paused, not yet due, or the balance is insufficient. |

## Architecture: pool-adapter cross-contract calls, not SDEX

Soroban contracts cannot interact with the classic Stellar SDEX — there is no
host function for it. Scheduled swaps therefore go through a
contract-to-contract call into an AMM/liquidity-pool-style contract instead.
`execute_swap` calls a small internal `SwapPool` trait rather than a fixed
Soroban `#[contractclient]`, so different pool integrations can be swapped in
later; today there's one implementation, `GenericPoolAdapter`, which targets
pools exposing a generic `swap(to, token_in, token_out, amount_in,
min_amount_out) -> i128` entrypoint (every transfer in that flow is a direct,
self-authorizing call by whichever contract currently holds the funds being
moved, so no deeper cross-contract authorization plumbing is needed). See the
doc comment on `GenericPoolAdapter` in `contracts/dca-vault/src/lib.rs` for
why this doesn't reuse soroban-examples' `liquidity_pool` interface directly
(its fixed two-asset, exact-output design doesn't fit a generic vault).

## Building and testing

```sh
cargo test
cargo build --target wasm32v1-none --release
```

Note: `wasm32-unknown-unknown` is not supported by `soroban-sdk` 26.1.0 on
Rust 1.82+; use `wasm32v1-none` instead.

## Deployment

Not yet deployed. Testnet deployment is pending review of the core vault and
swap execution logic in this repo, and integration testing against a real
deployed pool contract (tests so far run against an in-repo mock pool).
