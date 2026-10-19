# Changelog

## Unreleased

### Added
- **docs:** a [mainnet launch checklist](docs/developers/mainnet-checklist.md) covering code, keys, parameters, monitoring, communication and rollback. Every item names an owner and the evidence required, and it is linked from the README, the docs index and `SECURITY.md`. It records that `scripts/deploy.sh` is testnet-only because it deploys a test USDC asset.
- **docs:** a [multisig admin guide](docs/developers/multisig-admin.md) creates a 2-of-3 Stellar account with the CLI, tests the policy, disables the master key, hands registry, voucher and receipt to it through the two-step handover, and shows how to submit admin calls with two signatures. Linked from the mainnet checklist and the contract reference.
- **docs:** an [integration guide](docs/developers/integration.md) for wallets and remittance apps, with runnable JavaScript for deriving `beneficiary_ref` without leaking identity, quoting the fee, funding a voucher, reading status and reading receipts directly from the contracts, including error handling and a privacy checklist.
- **test:** `multisig_admin_can_act_on_all_three_contracts` hands registry, voucher and receipt to one multisig-style account and performs an admin action on each, covering the handover and the admin calls the guide documents.
- **ci:** a `coverage` job runs `cargo llvm-cov`, prints the coverage table in the job output and fails when the contract sources drop below **98% of lines** or **95% of regions** (today: 98.14% and 95.99%). Test files are excluded from the measurement, so the number says how much of the *contract* code the tests reach. The `lcov` report is uploaded as an artefact, and the README links the job.
- **ci:** a `dependency audit` job runs `cargo deny check` — RustSec advisories, the licence allow list, the duplicate and wildcard bans and the source allow list — against a new `deny.toml`. It carries no exceptions today; any future one has to carry a reason.
- **ci:** a `release` workflow builds the contracts when a `v*` tag is pushed, optimises them with `wasm-opt -Oz` (binaryen 133) and attaches them to the GitHub release with a `SHA256SUMS` file and release notes that explain how to compare the hash with a deployed contract.
- **scripts:** `build-wasm.sh` builds every contract the one way the release, the deployment and the verification agree on, and writes `SHA256SUMS` next to them. `verify-deployments.sh` fetches each deployed contract with `stellar contract fetch` and compares it with a local build twice: by SHA-256, and by contract interface (functions, errors, events) as the Stellar host reads it. It exits non-zero when either differs.
- **docs:** the README publishes the verification result for testnet and explains it. Three of the four deployed contracts expose exactly the interface this source builds; the voucher does not, because its lifecycle events gained `service_code` and `amount` after it was deployed. Redeploying fixes it.

### Changed
- **scripts:** `scripts/deploy.sh` now refuses to overwrite an existing `deployments/<network>.env` unless `--force` is passed, so an accidental second run cannot replace the live addresses. `--dry-run` prints every command without building, deploying or sending a transaction, and the final summary makes the outcome explicit. `scripts/test-deploy.sh` covers the refusal, `--force` and dry-run paths without a network, and the new `deploy script` CI job runs it on every pull request.
- **deploy:** `scripts/deploy.sh` and `scripts/deploy-faucet.sh` deploy the wasm that `scripts/build-wasm.sh` produces, so what is deployed is what is released. The faucet used to be built on its own (`cargo build -p sci-faucet`), which links a different `soroban-sdk` than the rest of the workspace and produces different bytes.
- **build:** the build takes the cargo registry path out of the wasm (`--remap-path-prefix`). The Soroban SDK embeds the absolute path of its own source in its panic messages, so the artefact used to carry the builder's home directory, and two machines never agreed on its hash.
- **cargo:** the workspace path dependencies carry a version, so `cargo deny` does not read them as wildcard version requirements.
- **voucher:** `voucher_claimed`, `voucher_attested`, `voucher_disputed`, `voucher_settled` and `voucher_refunded` now carry `service_code` and `amount` in their data, appended after the fields each event already had. Only `voucher_created` used to say what the voucher was for, so an indexer or wallet reading a single lifecycle event had to load the voucher to show what was paid for. `voucher_refunded` already carried `amount`; it gains `service_code` only. Topics are unchanged and the data map is keyed by name, so the indexer in `sci-healthcare-backend` decodes the new layout without a breaking change (its matching decoder change is linked from the pull request). Covered by five event snapshot tests in `contracts/voucher/src/test.rs` that read the decoded XDR event and the regenerated `test_snapshots`.

## 0.2.0 — 2026-10-09

Redeployed to testnet on 2026-10-07; addresses are in `deployments/testnet.env`.

### Fixed
- **voucher:** a `Claimed` voucher that is never attested no longer locks the funder's money forever. `refund` accepts it seven days (`CLAIM_GRACE_SECS`) after expiry.
- **voucher:** a provider suspended after a voucher was funded can no longer `claim` it.
- **voucher:** every state transition extends the instance TTL, not only `create_voucher`.
- **registry:** attester entries are TTL-extended on read like providers and services.

### Added
- "Try it in five minutes" walkthrough in the README.
- Issue forms (bug, feature) and a pull request template; blank issues off, security reports routed to SECURITY.md.
- **faucet** (testnet only): anyone can get 50 test USDC once a day from the app, with no server holding the issuer key. The demo token's SAC names the faucet as admin; `release_token_admin` hands it back. Deployed by `scripts/deploy-faucet.sh`, which refuses mainnet. 10 tests.
- Two-step admin handover (`propose_admin` / `accept_admin` / `get_pending_admin`) in registry, voucher and receipt. Replaces the registry's one-step `set_admin`; the voucher admin previously could not be rotated at all.
- `refundable_at(voucher_id)` view on the voucher contract.
- Documented dispute reason codes 1–4.

Tests: 94 (was 69).

## 0.1.0 — 2026-09-02

Initial registry, voucher escrow and care receipt contracts.
