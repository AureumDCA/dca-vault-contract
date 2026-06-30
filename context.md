# StellarDCA — Project Context Log

This file tracks every edit, decision, and development session across the
StellarDCA project. Update it at the end of every working session — newest
entry on top.

## Project structure

StellarDCA is split across three independent git repos, all under the
`StellarDCA` GitHub org:

- **dca-vault-contract** (this repo) — Trustless DCA vault on Stellar Soroban,
  automated dollar-cost averaging with SDEX execution. Also holds this
  cross-project context log.
- **dca-vault-backend** — Schedule executor, price-history indexer, and
  portfolio API (Node/TypeScript).
- **dca-vault-frontend** — Vault creation, dashboard, and portfolio UI
  (Node/TypeScript).

Each repo is committed and pushed independently, one repo at a time.

## Session log

### 2026-06-30

- Discovered the workspace already contained three initialized repos
  (`dca-vault-backend`, `dca-vault-contract`, `dca-vault-frontend`), each with
  a single `chore: initialize workspace` commit and a README stub, already
  pushed to `github.com/StellarDCA/*`.
- Initially set up a fourth root-level repo (`StellarDCA/StellarDCA`) to hold
  this context log, plus a `.gitignore` and VS Code settings at the root.
- Reconsidered: removed the root repo entirely (no 4th repo). Moved this
  `context.md` into `dca-vault-contract` instead, and added a `.gitignore` to
  each of the three repos individually.
- Added `.vscode/settings.json` at the workspace root (not part of any repo)
  so VS Code's Source Control activity bar surfaces all three repos.
