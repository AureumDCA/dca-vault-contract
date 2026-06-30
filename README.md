# DCA Vault Contract

A Soroban smart contract on Stellar implementing a trustless Dollar-Cost
Averaging (DCA) vault: users deposit XLM into a personal on-chain vault,
configure a recurring swap schedule, and the contract tracks (and will
eventually execute) those scheduled swaps without a custodian.

## The DCA Vault concept

Dollar-cost averaging means buying a fixed amount of an asset on a fixed
schedule (e.g. weekly) instead of all at once, smoothing out entry price over
time. This contract gives every depositor their own on-chain `Vault`:

- **Balance** — XLM the user has deposited but not yet committed to a
  scheduled swap.
- **Schedule** — an optional, user-configured recurring purchase: how often
  (`Daily` / `Weekly` / `Monthly`), how much per execution, and which asset
  to buy.
- **Paused flag** — lets the owner halt scheduled execution without
  withdrawing funds or deleting the schedule.

The vault only tracks state today; it does not yet move funds on a schedule
(see Architecture below).

## Functions

| Function | Description |
| --- | --- |
| `initialize(token)` | One-time setup: records the token contract (XLM's Stellar Asset Contract) that deposits/withdrawals move. |
| `deposit(owner, amount)` | Owner-authorized. Transfers `amount` XLM from `owner` into the vault, increasing its balance. |
| `withdraw(owner, amount)` | Owner-authorized. Decreases the vault balance and transfers `amount` XLM back to `owner`. Panics if `amount` exceeds the balance. |
| `create_schedule(owner, frequency, amount_per_execution, target_asset)` | Owner-authorized. Creates or replaces the owner's recurring swap schedule. |
| `pause_schedule(owner)` | Owner-authorized. Halts scheduled execution without clearing the schedule. |
| `resume_schedule(owner)` | Owner-authorized. Re-enables a paused schedule. |
| `get_vault(owner)` | Read-only. Returns the owner's `Vault`, or panics if none exists. |

Swap execution itself (actually buying `target_asset` on schedule) is not
implemented yet — see `// TODO: swap execution adapter (next feature)` in
`contracts/dca-vault/src/lib.rs`.

## Architecture: SDEX first, Swyft adapter later

Scheduled swaps will execute against Stellar's built-in SDEX (the native
on-chain order book/AMM, reachable from a Soroban contract without any
external dependency) first. A pluggable adapter for routing through other
liquidity sources (e.g. Swyft) is planned as a follow-up, once the core vault
logic above is tested and deployed — keeping the initial execution path
simple and fully on-chain before adding optional, potentially
better-priced routing.

## Building and testing

```sh
cargo test
cargo build --target wasm32v1-none --release
```

Note: `wasm32-unknown-unknown` is not supported by `soroban-sdk` 26.1.0 on
Rust 1.82+; use `wasm32v1-none` instead.

## Deployment

Not yet deployed. Testnet deployment is pending completion and review of the
core vault logic (this repo) and the swap execution adapter (next feature).
