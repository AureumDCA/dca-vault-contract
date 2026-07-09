# AureumDCA — Project Context Log

This file tracks every edit, decision, and development session across the
AureumDCA project. Update it at the end of every working session — newest
entry on top. Sessions are numbered sequentially (Session 1, Session 2, ...).

## Project structure

AureumDCA is split across three independent git repos, all under the
`AureumDCA` GitHub org:

- **dca-vault-contract** (this repo) — Trustless DCA vault on Stellar Soroban,
  automated dollar-cost averaging executed via contract-to-contract calls
  into AMM/pool contracts (not SDEX — see Session 3, Soroban has no host
  function for the classic SDEX). Also holds this cross-project context log.
- **dca-vault-backend** — Schedule executor, price-history indexer, and
  portfolio API (Node/TypeScript).
- **dca-vault-frontend** — Vault creation, dashboard, and portfolio UI
  (Node/TypeScript).

Each repo is committed and pushed independently, one repo at a time.

## Session log

### Session 10 — 2026-07-09

**README: CI/License badges.** Added `CI` and `License: MIT` shields.io
badges directly under the H1 title, linking to the repo's `ci.yml` workflow
and the MIT license text. The "Deployment" section already had an
**Explorer** table row linking to the same Stellar Expert testnet contract
page requested for today's task, so it was left as-is rather than adding a
redundant second link — additive-only, no rewording/restructuring of existing
sections. `cargo test` (14/14) and `cargo build --target wasm32v1-none
--release` both still pass clean after the README-only change.

### Session 9 — 2026-07-08

**Closed issue #3: `ScheduleCreated` event for executor discoverability.**

Added a `ScheduleCreated` event, emitted at the end of `create_schedule`, so
the backend indexer can discover a vault the moment it's scheduled instead of
waiting for its first `SwapExecuted` event. Follows the exact same
`#[contractevent]` pattern as `SwapExecuted` (Session 4):

- `#[contractevent(topics = ["schedule_created"])]` struct `ScheduleCreated`
  with `#[topic] owner: Address` plus `frequency`, `amount_per_execution`,
  `target_asset`, `pool_address` — matching the field set the backend's
  `dca-vault-backend` poller (already merged, blocked on this event existing)
  expects. The `"schedule_created"` topic string is 16 chars, too long for
  `symbol_short!`, so the test builds it with `Symbol::new(&env, ...)` instead
  — same as the contract macro does internally.
- `create_schedule` now clones `owner`, `target_asset`, and `pool_address`
  before moving the originals into the `Schedule` struct / storage key, so the
  same values can be reused to construct and `.publish()` the event afterward.
- New test `create_schedule_emits_schedule_created_event`, modeled on
  `execute_swap_succeeds_when_due`'s event-assertion pattern: captures
  `env.events().all().filter_by_contract(&contract_id)` right after the call,
  and asserts against a `Map<Symbol, Val>` with keys in alphabetical order
  (`amount_per_execution, frequency, pool_address, target_asset`).

`cargo test`: 14 passed, 0 failed (13 previous + this one). `cargo build
--target wasm32v1-none --release`: succeeds, zero warnings. README and
CONTRIBUTING.md updated (test count 13 → 14, new test listed, `create_schedule`
function-table row now mentions the event).

### Session 8 — 2026-07-03

**Day 2 — GitHub issue creation.** Opened the contract repo's backlog as
tracked GitHub issues so the work is discoverable by Stellar Drips Wave
contributors. Added the missing `Stellar Wave` label (blue, `#0075ca`) and
created 4 issues, each with a Description / Tasks / Acceptance Criteria
structure:

- **#1** Implement Swyft concentrated liquidity adapter for `execute_swap`
  (`enhancement`, `help wanted`) — second `SwapPool` trait impl alongside
  `GenericPoolAdapter`, with a per-`Schedule` `adapter` enum selector.
- **#2** Add price oracle integration for accurate `min_amount_out`
  (`enhancement`, `help wanted`) — replaces the naive 1:1 slippage baseline
  (the existing code TODO) with a real quote/oracle source.
- **#3** Add vault creation event for executor discoverability
  (`enhancement`, `good first issue`) — emit `ScheduleCreated` via
  `#[contractevent]` so the backend can index brand-new vaults before their
  first swap. Pairs with the backend indexer work.
- **#4** Expand test coverage — edge cases and multi-execution scenarios
  (`enhancement`, `good first issue`) — all three frequency variants,
  sequential swaps, schedule replacement, token-level withdraw.

Complexity/points labels are intentionally left off — the maintainer assigns
those manually. No code changes this session.

### Session 7 — 2026-07-02

**Documentation pass across all three AureumDCA repos.**

For this repo:
- Added `CONTRIBUTING.md`: prerequisites (Rust 1.91.0+, wasm32v1-none target, soroban-sdk 26.1.0), getting started commands, full list of 13 tests with one-line descriptions, branch naming conventions, Conventional Commits style guide, PR checklist, issue label glossary, note that complexity/points labels are maintainer-only, Drips Wave rules (don't resolve others' issues, don't inflate labels).
- Updated `README.md`: added test suite summary (13 tests, what they cover), Contributing section linking to CONTRIBUTING.md.

**Full contract feature state as of today:**

*Functions*: `initialize`, `deposit`, `withdraw`, `create_schedule`, `pause_schedule`, `resume_schedule`, `get_vault`, `execute_swap`.

*Tests (13)*: `deposit_increases_balance`, `get_vault_with_no_schedule_returns_none`, `withdraw_decreases_balance`, `withdraw_more_than_balance_panics`, `create_schedule_attaches_schedule`, `pause_and_resume_schedule_toggle_paused`, `get_vault_on_nonexistent_owner_panics`, `execute_swap_succeeds_when_due`, `execute_swap_panics_when_not_due`, `execute_swap_panics_when_paused`, `execute_swap_panics_when_balance_insufficient`, `execute_swap_is_callable_by_non_owner`, `execute_swap_pool_failure_is_atomic`.

*Deployment*: Testnet, contract `CDJF7V5NLGKAV7RHTBCR3LMHC7MUS7IWL6KYSLO6ZWEEJYJGWUVGEDEO`, initialized 2026-07-01 with XLM SAC `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`.

*Key technical decisions* (consolidated):
- **wasm32v1-none**: soroban-sdk 26.1.0 rejects `wasm32-unknown-unknown` on Rust 1.82+; use `wasm32v1-none` (requires Rust 1.84+; effective MSRV of the full dep tree is 1.91.0).
- **`Option<Schedule>` — hand-rolled TryFromVal**: soroban-sdk 26.1.0's `#[contracttype]` derive can't handle `Option<OtherContractTypeStruct>` fields under `cargo test` (testutils feature unification triggers an infallible Into<ScVal> requirement the nested struct can't satisfy). `Vault` is hand-implemented with `TryFromVal`/`Val` and requires Map keys in strict alphabetical order (`balance, owner, paused, schedule`).
- **SDEX is not callable from Soroban**: no host function exists for it. Swap execution uses contract-to-contract calls via `GenericPoolAdapter` targeting `swap(to, token_in, token_out, amount_in, min_amount_out) -> i128`. Push-then-call pattern: vault pushes `token_in` to pool first (self-authorizing), then calls `pool.swap`; if pool panics, the push reverts atomically.
- **`#[contractevent]`**: SwapExecuted uses the macro (Session 4). Map key sort is alphabetical; topics are `["swap"]` + owner address.

---

### Session 6 — 2026-07-01

**dca-vault-frontend scaffold (Next.js 16 / Tailwind / Freighter)**

Initialized `dca-vault-frontend` using `create-next-app@latest` (TypeScript, Tailwind, ESLint, app router, no `src/` dir). Additional deps: `@stellar/freighter-api @stellar/stellar-sdk`.

**`lib/freighter.ts`** — typed wrappers for Freighter v5 API (`isConnected`, `connect`, `getPublicKey`, `signTransaction`). Key v5 quirks: `getPublicKey` was removed — use `getAddress()` which returns `{ address }`. `isConnected()` returns `{ isConnected: boolean }`, not a bare boolean. `requestAccess()` returns `{ address }`. `signTransaction()` returns `{ signedTxXdr, signerAddress }`.

**`lib/stellar.ts`** — fetch wrappers around the backend REST API (`getVault`, `getHistory`, `getPerformance`). `NEXT_PUBLIC_API_URL` env var, defaults to `http://localhost:3001`.

**Components** (in `app/components/`):
- `ConnectWallet.tsx` — calls `isConnected()` then `connect()`, props `{ onConnect: (pk: string) => void }`
- `VaultStatus.tsx` — displays balance (stroops ÷ 1e7 → XLM), schedule details, paused badge. `formatFrequency()` handles `#[contracttype]` enum shape: `scValToNative` converts C-like enum to `["VariantName"]` (a single-element array), so check `Array.isArray(freq) ? freq[0] : freq`.
- `CreateSchedule.tsx` — form with frequency select (Daily/Weekly/Monthly), amountPerExecution, targetAsset, poolAddress, minAmountOutBps. Props `{ onSubmit: (values) => Promise<void> }`.
- `SwapHistory.tsx` — table of `SwapEvent[]` (ledger, amount_in, amount_out, tx_hash).

**Pages**:
- `app/page.tsx` — landing page; after Freighter connect, routes to `/vault?owner=<pk>`.
- `app/vault/page.tsx` — `VaultDashboard` (uses `useSearchParams`, loads vault + history in parallel via `Promise.all`). Split into `VaultDashboard` + `VaultPage` wrapper because Next.js app router requires `useSearchParams()` inside a `Suspense` boundary — any component using it must be a leaf wrapped in `<Suspense>` by its parent.

**Non-obvious fixes**:
- `create-next-app` refuses any non-`.git` file in the target directory — moved `README.md` to `/tmp/` during init, restored after.
- `vault !== null` guard (not `vault &&`) required because `vault: unknown` — the `&&` short-circuit returns `unknown` on the falsy path, which isn't `ReactNode`.
- `.gitignore` had `.env*` catching `.env.local.example`; fixed with `!.env*.example` negation.

**Env vars**: `NEXT_PUBLIC_API_URL`, `NEXT_PUBLIC_CONTRACT_ID`, `NEXT_PUBLIC_NETWORK_PASSPHRASE`. `.env.local` filled with testnet values (gitignored); `.env.local.example` committed.

**CI** (`.github/workflows/ci.yml`): `npm ci` → `npx tsc --noEmit` → `npm run build`. Uses `npx tsc` directly (not `npm run typecheck`) because Next.js sets `noEmit: true` in `tsconfig.json` and `npm run build` also runs tsc internally. CI run `28536253231`: completed/success in 35s.

**Pending**: `CreateSchedule.onSubmit` in `vault/page.tsx` is a stub (`console.log + alert`) — Freighter signing + stellar-sdk transaction building for `create_schedule` not yet implemented.

Three commits pushed: `chore: initialize dca-vault-frontend with Next.js and Tailwind`, `feat: scaffold vault UI components and Freighter integration`, `ci: add GitHub Actions workflow for typecheck and build`.

---

### Session 5 — 2026-07-01

**dca-vault-backend scaffold (Node.js / Express / TypeScript)**

Initialized `dca-vault-backend` with `package.json`, `tsconfig.json`, and deps: `express @stellar/stellar-sdk better-sqlite3 dotenv node-cron` (runtime) + `typescript ts-node @types/*` (dev). Scripts: `build` (`tsc`), `dev` (`ts-node src/index.ts`), `start` (`node dist/index.js`), `typecheck` (`tsc --noEmit`).

**`src/config.ts`** — loads all env vars, throws on missing required ones, exports typed `Config`.

**`src/indexer/db.ts`** — better-sqlite3 (synchronous, WAL mode). Tables: `swap_events` with `UNIQUE(tx_hash, owner)` for deduplication via `INSERT OR IGNORE`; `indexer_state` for cursor tracking. Exported `Db` interface: `insertSwapEvent`, `getSwapEvents`, `getAllOwners` (SELECT DISTINCT), `getLastLedger`, `setLastLedger`, `close`.

**`src/indexer/poller.ts`** — polls Soroban RPC for SwapExecuted events. Topic filter: `xdr.ScVal.scvSymbol("swap").toXDR("base64")` = `AAAADwAAAARzd2Fw`. On first run (lastLedger=0), starts from `latestLedger - 100`. Parses events via `scValToNative`. Event data is a Map with keys `amount_in`, `amount_out`, `pool_address` (alphabetically sorted, as the `#[contractevent]` macro sorts Map keys before `map_new_from_slices`).

**`src/executor/executor.ts`** — `getVaultState()` simulates `get_vault`; `executeSwap()` simulates → `rpc.assembleTransaction` → sign → `sendTransaction` → polls `getTransaction` up to 20× with 3s delay. Known limitation (TODO): executor only discovers owners via prior swap events; `create_schedule` events not yet indexed, so brand-new vaults aren't executed until after their first swap.

**`src/api/`** — Express v5 router with handlers for: `GET /health`, `GET /vaults/:owner` (simulates `get_vault`, returns `scValToNative(retval)`), `GET /vaults/:owner/history` (SQLite), `GET /vaults/:owner/performance` (avg_price = total_invested / total_received). Express v5 types `req.params` values as `string | string[]`, requires `Array.isArray` guard per param.

**stellar-sdk v15.1.0 API shape** (non-obvious): RPC client is `rpc.Server` (not `SorobanRpc`); `scValToNative` is top-level; `rpc.assembleTransaction`; `rpc.Api.isSimulationSuccess()` type guard; `rpc.Api.GetTransactionStatus` enum.

**CI** (`.github/workflows/ci.yml`): `npm ci` → `npm run typecheck` → `npm run build`. Node 20, `actions/setup-node@v4` with `cache: 'npm'`. CI confirmed green.

Two commits pushed: `feat: scaffold dca-vault-backend with executor, indexer, and API`, `ci: add GitHub Actions workflow for typecheck and build`.

---

### Session 4 — 2026-07-01

**Two tasks completed.**

**Task 1 — migrate swap event to `#[contractevent]`**

Replaced the deprecated `env.events().publish((symbol_short!("swap"), owner), (amount_in, amount_out, pool_address))` call with a typed `SwapExecuted` struct annotated with `#[contractevent(topics = ["swap"])]`. Topics unchanged: static `"swap"` + `owner` address (marked `#[topic]`). Data changes from a raw tuple/Vec to a Map — the macro's default `data_format`, which is better for tooling/SDKs. The macro sorts Map keys alphabetically before calling `map_new_from_slices`; for our fields `amount_in`, `amount_out`, `pool_address` the declaration order is already alphabetical so no reordering needed. Updated the event assertion in `execute_swap_succeeds_when_due` to construct an equivalent `Map::<Symbol, Val>::from_array(...)` for comparison. Removed `symbol_short` from lib.rs imports (only used in the old `.publish` call); added it directly to test.rs since `use super::*` no longer propagates it. CI confirmed green. Also added `execute_swap_pool_failure_is_atomic` test (confirmed in the previous session's final task): verifies that when MockPoolFailing panics mid-execution, the pre-swap token push and all state mutations revert atomically — checks both the accounting layer (`vault.balance`) and the actual token balances held by the vault and pool contracts.

**Task 2 — Stellar Testnet deployment**

Deployed to Stellar Testnet 2026-07-01 using the pre-existing `deployer` key (`GAODBHVR63Z56MVQRBEJSYM2H5423LJ4WAPUUBOFG4JYY72S6ROKVZRX`), which was already funded via friendbot.

- **Contract ID**: `CDJF7V5NLGKAV7RHTBCR3LMHC7MUS7IWL6KYSLO6ZWEEJYJGWUVGEDEO`
- **Network**: Stellar Testnet
- **XLM SAC**: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`
- **Explorer**: https://stellar.expert/explorer/testnet/contract/CDJF7V5NLGKAV7RHTBCR3LMHC7MUS7IWL6KYSLO6ZWEEJYJGWUVGEDEO

`initialize(token = XLM_SAC)` called immediately after deploy. Contract is live and ready for `deposit` / `create_schedule` / `execute_swap` calls. `execute_swap` requires a pool contract also deployed on testnet — no public pool yet targeting our `GenericPoolAdapter` ABI; that's the next integration step.

Note: the `godamongstmen897` GitHub account owns the `AureumDCA` org — not `N-thnI` (the default active gh account). Switch with `gh auth switch -u godamongstmen897` before pushing to any `AureumDCA/*` repo.

### Session 1 — 2026-06-30

**Repo/workspace setup**

- Discovered the workspace already contained three initialized repos
  (`dca-vault-backend`, `dca-vault-contract`, `dca-vault-frontend`), each with
  a single `chore: initialize workspace` commit and a README stub, already
  pushed to `github.com/AureumDCA/*`.
- Initially set up a fourth root-level repo (`AureumDCA/AureumDCA`) to hold
  a context log, plus a `.gitignore` and VS Code settings at the root.
  Reconsidered: removed that root repo entirely (no 4th repo). Moved
  `context.md` into `dca-vault-contract` instead, and added a `.gitignore` to
  each of the three repos individually.
- Added `.vscode/settings.json` at the workspace root (not part of any repo)
  so VS Code's Source Control activity bar surfaces all three repos.

**What was built**

Scaffolded the `dca-vault` Soroban contract as a standard workspace:
workspace `Cargo.toml` + `contracts/dca-vault` member, `soroban-sdk` 26.1.0
(latest stable). Implemented the `Vault` / `Schedule` / `Frequency` data
model and all six core contract functions: `initialize` (records the XLM
token contract address — needed since deposit/withdraw move real tokens but
the spec's function signatures don't pass a token address), `deposit`,
`withdraw`, `create_schedule`, `pause_schedule`, `resume_schedule`,
`get_vault`. Swap execution itself is stubbed with
`// TODO: swap execution adapter (next feature)` in `create_schedule`.

**Test count / build status**

- `cargo test`: **6 passed, 0 failed** (deposit, withdraw, over-withdraw
  panic, schedule creation, pause/resume, `get_vault` on missing owner
  panic).
- `cargo build --target wasm32-unknown-unknown --release`: **fails** on this
  toolchain (see Toolchain note below) — not a code bug.
- `cargo build --target wasm32v1-none --release`: **succeeds**, produces
  `target/wasm32v1-none/release/dca_vault.wasm`.

**Key decisions**

- **Architecture: SDEX first, Swyft adapter later.** Scheduled swaps will
  execute against Stellar's built-in SDEX first (native, reachable from a
  Soroban contract with no external dependency), keeping the initial
  execution path simple and fully on-chain. A pluggable adapter for other
  liquidity sources (e.g. Swyft) is deferred until the core vault logic here
  is tested and deployed — avoids coupling the vault's accounting to an
  external integration before the basics are proven.
- **`Vault` is hand-implemented, not `#[contracttype]`.** soroban-sdk
  26.1.0's `#[contracttype]` derive can't handle a struct field typed
  `Option<OtherContractTypeStruct>` — it generates a testutils-only XDR/ScVal
  conversion that requires an infallible `Into<ScVal>` the custom struct
  doesn't have. This only surfaces under `cargo test` (dev-dependency feature
  unification turns on `testutils`), not under the production `cargo build
  --release`. Since `Vault` needs `schedule: Option<Schedule>` per spec,
  `Vault` is hand-implemented (manual `TryFromVal`/`Val` impls replicating
  only the runtime Env/Val conversion) instead of using `#[contracttype]`.
  Map keys for the manual impl **must be alphabetically sorted** (`balance,
  owner, paused, schedule`) or `env.map_new_from_slices` panics with "ScMap
  was not sorted by key".
- **Toolchain note**: with Rust 1.95 + soroban-sdk 26.1.0, the target
  `wasm32-unknown-unknown` is rejected at build-script time ("Rust compiler
  1.82+ ... is unsupported by the Soroban Environment, use 'wasm32v1-none'
  available with Rust 1.84+"). Use `cargo build --target wasm32v1-none
  --release` instead.
- **Cargo.lock is committed**, not gitignored — this is a deployable
  contract, not a published library, so locking dependency versions keeps
  WASM builds reproducible. Added `test_snapshots/` to `.gitignore` instead
  (auto-generated by `cargo test` on panicking tests, not meant to be
  committed).
- Verified end-to-end in a scratch project before touching the real repo:
  `register_stellar_asset_contract_v2(admin) -> StellarAssetContract`
  (`.address()` for the token id) is the current testutils API (the older
  `register_stellar_asset_contract` is deprecated). `token::TokenClient`
  (not `token::Client`, which is now a deprecated alias) is used for
  transfers; `token::StellarAssetClient::new(env, &addr).mint(...)` funds
  test accounts.

Committed and pushed to `dca-vault-contract`.

### Session 2 — 2026-06-30

Added GitHub Actions CI (`.github/workflows/ci.yml`): runs on push to `main`
and on `pull_request`, runs `cargo test` (native target, correctness) then
`cargo build --target wasm32v1-none --release` (deployability), with
`Swatinem/rust-cache` for speed. Confirmed broken
`wasm32-unknown-unknown` (see Session 1) is not used anywhere in the
workflow.

**Key decision — Rust 1.91.0, not 1.84.0.** The wasm32v1-none *target*
exists from Rust 1.84+, but that's not the same as soroban-sdk 26.1.0's
actual minimum supported Rust version. First CI attempt pinned 1.84.0 and
failed: a transitive dependency (`enum-ordinalize-derive`) needs the
`edition2024` Cargo feature, which requires Cargo 1.85+. Tried 1.85.0 next
(locally, before pushing again) — still failed: `cargo +1.85.0 test` reported
soroban-sdk 26.1.0 and several of its deps (`darling` 0.23, `enum-ordinalize`
4.4.1, `serde_with` 3.21) declare rustc 1.88–1.91 as their minimum. Settled
on 1.91.0, verified locally with both `cargo +1.91.0 test` and
`cargo +1.91.0 build --target wasm32v1-none --release` before pushing again.
CI run `28464308119`: completed/success.

Lesson: "the target exists as of version X" and "the SDK's MSRV is X" are
different facts — check the latter (e.g. by trying the pin locally with
`cargo +<version>`) rather than assuming the former is sufficient.

### Session 3 — 2026-06-30

**What was built**: scheduled swap execution. `Schedule` gained
`pool_address: Address` and `min_amount_out_bps: u32`; `create_schedule` now
takes both (set once per schedule, not changeable independently). Added a
`SwapPool` trait (internal abstraction, not a `#[contractclient]`) and one
implementation, `GenericPoolAdapter`, plus the permissionless
`execute_swap(owner) -> i128` entrypoint: validates not-paused/due/balance,
calls the adapter, updates `last_execution_ledger`/`next_execution_ledger`
and balance, emits a `swap` event (`env.events().publish`, topics
`(symbol_short!("swap"), owner)`, data `(amount_in, amount_out,
pool_address)` — this is the shape `dca-vault-backend`'s indexer will read),
and returns the amount received.

**Why SDEX was dropped.** The original plan (see Session 1's README note)
was "SDEX first, Swyft adapter later." That's not implementable: Soroban
contracts have no host function for the classic Stellar SDEX at all — not a
priority/ordering question, a hard capability gap confirmed via Stellar's own
docs. Swaps must go through a contract-to-contract call into an AMM/pool
contract instead, so the architecture is now "pool-adapter cross-contract
calls" from the start, with `GenericPoolAdapter` as the first (only) adapter.

**The interface assumption was wrong, too.** Before writing `GenericPoolAdapter`,
cloned `stellar/soroban-examples` and actually read `liquidity_pool/src/lib.rs`
rather than guessing. Its real `swap` signature is
`swap(e, to: Address, buy_a: bool, out: i128, in_max: i128)` — a fixed
two-token (A/B) pool, exact-*output* amounts, `to.require_auth()` required,
no return value. That doesn't fit a vault that needs to swap into an
arbitrary `target_asset` by exact *input* amount. So `GenericPoolAdapter`
targets a different, simpler ABI we define ourselves: `swap(to, token_in,
token_out, amount_in, min_amount_out) -> i128`. The example was still useful
for the *patterns* (token::Client transfers, contract structure), just not
its literal signature.

**Avoiding deeper cross-contract auth.** A pool that pulls `token_in` from
the caller via `transfer(&to, ..)` requiring `to.require_auth()` (the
example's pattern) would need the vault to call
`env.authorize_as_current_contract(..)` before invoking the pool, since
Soroban only auto-authorizes a contract's *direct* calls, not calls two hops
deep (vault → pool → token). To avoid that complexity, `GenericPoolAdapter`
uses a push-then-call convention instead: the vault transfers `amount_in` of
`token_in` to the pool itself first (a direct, self-authorizing call), then
invokes `pool.swap(to = vault's own address, ...)`; the pool is expected to
pay `token_out` back to `to` via its own direct, self-authorizing transfer.
Every transfer in the flow is a contract moving its own already-held funds,
so no deeper auth plumbing is needed. If the pool's `min_amount_out` check
fails and it panics, the whole transaction — including the earlier push —
reverts atomically, so funds can't get stuck mid-swap.

**Slippage math kept naive on purpose**: `min_amount_out = amount_in *
min_amount_out_bps / 10_000` assumes a naive 1:1 expected output, flagged
with a TODO in `execute_swap` to replace with a real price/impact
calculation (oracle or pool quote) later — explicitly out of scope for this
pass per the spec.

**Mock pool testing approach**: `execute_swap` can't be tested without
something to swap against, so `test.rs` defines a `MockPool` contract
(`#[cfg(test)]`-only, lives in the test module) implementing the
`GenericPoolAdapter` target ABI with a fixed 1:1 rate, pre-funded with
`target_asset` liquidity via `StellarAssetClient::mint`. Two non-obvious test
fixes along the way: `env.events().all()` only returns events from the *last*
invocation and must be captured immediately after `execute_swap`, before any
other client call (e.g. a follow-up `get_vault`); and it returns *every*
event from the call tree (including the underlying token `transfer` events
from both legs of the swap), so the assertion uses
`.filter_by_contract(&contract_id)` to isolate the vault's own `swap` event.

7 new tests added (12 total): `execute_swap_succeeds_when_due` (balance,
ledgers, and filtered event all asserted), `execute_swap_panics_when_not_due`,
`execute_swap_panics_when_paused`, `execute_swap_panics_when_balance_insufficient`,
and `execute_swap_is_callable_by_non_owner` (proves permissionless triggering
by calling `env.set_auths(&[])` right before `execute_swap` — disables auth
mocking entirely, so the call only succeeds because nothing in the path
actually requires anyone's signature).

`cargo test`: 12 passed, 0 failed. `cargo build --target wasm32v1-none
--release`: succeeds (one pre-existing deprecation warning: `env.events().publish`
is deprecated in favor of `#[contractevent]` in this soroban-sdk version;
kept `publish` since the spec asked for it explicitly — worth revisiting
later).

README and this log updated to drop the SDEX-first framing and describe the
pool-adapter architecture instead.
