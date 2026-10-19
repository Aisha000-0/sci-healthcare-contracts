# Changelog

## Unreleased

### Changed
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
