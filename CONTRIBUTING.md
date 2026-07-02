# Contributing to dca-vault-contract

Welcome! This repo holds the Soroban smart contract at the core of StellarDCA — a trustless, permissionless DCA vault on Stellar. It's part of the **Stellar Drips Wave contributor program**, which rewards merged contributions with on-chain Drips payments. Every accepted PR earns you credit; maintainers assign complexity/points labels after review.

## Prerequisites

| Tool | Version |
| --- | --- |
| Rust | 1.91.0+ |
| Stellar CLI | latest |
| wasm32v1-none target | `rustup target add wasm32v1-none` |
| soroban-sdk | 26.1.0 (pinned in `Cargo.lock`) |

> **Note on targets:** `wasm32-unknown-unknown` is rejected by soroban-sdk 26.1.0 on Rust 1.82+. Always build with `wasm32v1-none`.

## Getting started

```sh
git clone https://github.com/StellarDCA/dca-vault-contract.git
cd dca-vault-contract
cargo build --target wasm32v1-none --release
cargo test
```

## Running tests

```sh
cargo test
```

The suite has **13 tests** covering:

- `deposit_increases_balance` — deposit correctly credits the vault balance
- `get_vault_with_no_schedule_returns_none` — vault with no schedule has `schedule: None`
- `withdraw_decreases_balance` — withdraw correctly debits the vault balance
- `withdraw_more_than_balance_panics` — over-withdrawal panics
- `create_schedule_attaches_schedule` — schedule fields are persisted correctly
- `pause_and_resume_schedule_toggle_paused` — pause/resume toggles the `paused` flag
- `get_vault_on_nonexistent_owner_panics` — reading a missing vault panics
- `execute_swap_succeeds_when_due` — a due, unpaused swap executes, balance and ledgers update, `swap` event emitted
- `execute_swap_panics_when_not_due` — execution before the next ledger panics
- `execute_swap_panics_when_paused` — execution on a paused schedule panics
- `execute_swap_panics_when_balance_insufficient` — execution with insufficient balance panics
- `execute_swap_is_callable_by_non_owner` — `execute_swap` is truly permissionless (auth disabled via `set_auths(&[])`)
- `execute_swap_pool_failure_is_atomic` — when the pool panics mid-swap, the prior token push reverts atomically

All tests must pass before opening a PR. Zero new warnings is also required.

## Building the WASM

```sh
cargo build --target wasm32v1-none --release
# output: target/wasm32v1-none/release/dca_vault.wasm
```

## Branch naming

| Prefix | Use for |
| --- | --- |
| `feat/` | New features |
| `fix/` | Bug fixes |
| `docs/` | Documentation only |
| `test/` | New or updated tests |
| `chore/` | Tooling, CI, dependency bumps |

## Commit style

This repo uses [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add vault pause reason field
fix: clamp min_amount_out to positive values
docs: update pool adapter interface notes
test: add execute_swap atomicity test for multi-hop pools
ci: pin Rust to 1.91.0
chore: bump soroban-sdk to 26.2.0
```

One logical change per commit. Keep subjects under 72 characters.

## PR checklist

Before requesting review, confirm:

- [ ] `cargo test` passes — all 13 (or more) tests green
- [ ] `cargo build --target wasm32v1-none --release` succeeds with no new warnings
- [ ] `context.md` updated with a brief note about what changed and why
- [ ] Branch name follows the naming conventions above
- [ ] Commit messages follow Conventional Commits

## Issue labels

| Label | Meaning |
| --- | --- |
| `bug` | Something isn't working |
| `documentation` | Improvements or additions to documentation |
| `duplicate` | This issue or pull request already exists |
| `enhancement` | New feature or request |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention is needed |
| `invalid` | This doesn't seem right |
| `question` | Further information is requested |
| `wontfix` | This will not be worked on |

**Do not add complexity or points labels yourself.** Maintainers assign these after review based on the scope of your contribution. Self-tagging inflates estimates and may disqualify the PR from Drips rewards.

## Stellar Drips Wave rules

- **Do not resolve issues you did not open.** Each contributor should work on their own issue. Closing someone else's issue without prior coordination will get your PR marked `invalid`.
- **Do not inflate complexity labels.** Requesting a higher-complexity label than the work warrants — or adding labels yourself — is against program rules and will be removed.
- If you have questions about scope or complexity, ask in the issue thread before writing code.
