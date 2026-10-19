---
title: Contract reference
nav_order: 10
---

# Contract reference

Three contracts. `registry` and `receipt` hold no references and deploy first; `voucher` points at both and deploys last.

All amounts are `i128` in the token's smallest unit. All timestamps are `u64` seconds since epoch.

---

## registry

Answers one question: who is allowed to do what.

### Types

```rust
pub enum ProviderStatus { Pending = 0, Active = 1, Suspended = 2 }

pub struct Provider {
    pub owner: Address,       // receives settlement
    pub name: String,
    pub country: String,      // ISO 3166-1 alpha-2
    pub status: ProviderStatus,
    pub registered_at: u64,
}

pub struct ServiceItem {
    pub code: u32,            // coarse category, never a diagnosis
    pub label: String,
    pub price: i128,
    pub active: bool,
}
```

### Functions

| Function | Parameters | Returns | Auth | Errors |
| --- | --- | --- | --- | --- |
| `initialize` | `admin: Address` | `()` | once | `AlreadyInitialized` |
| `register_provider` | `owner, name, country` | `()` | `owner` | `EmptyName`, `InvalidCountry`, `ProviderExists` |
| `set_provider_status` | `admin, provider_addr, status` | `()` | `admin` | `NotAuthorized`, `ProviderNotFound` |
| `upsert_service` | `provider_addr, code, label, price` | `()` | `provider_addr` | `InvalidPrice`, `EmptyName`, `ProviderNotActive` |
| `remove_service` | `provider_addr, code` | `()` | `provider_addr` | `ServiceNotFound` |
| `add_attester` | `admin, attester` | `()` | `admin` | `NotAuthorized` |
| `remove_attester` | `admin, attester` | `()` | `admin` | `NotAuthorized` |
| `propose_admin` | `admin, new_admin` | `()` | `admin` | `NotAuthorized` |
| `accept_admin` | `new_admin` | `()` | `new_admin` | `NoPendingAdmin`, `NotAuthorized` |
| `get_pending_admin` | — | `Option<Address>` | view | — |
| `get_provider` | `provider_addr` | `Provider` | view | `ProviderNotFound` |
| `is_active_provider` | `provider_addr` | `bool` | view | — |
| `get_service` | `provider_addr, code` | `ServiceItem` | view | `ServiceNotFound` |
| `get_services` | `provider_addr, codes: Vec<u32>` | `Vec<ServiceItem>` | view | — |
| `get_service_price` | `provider_addr, code` | `i128` | view | — |
| `is_attester` | `addr` | `bool` | view | — |
| `get_admin` | — | `Address` | view | `NotInitialized` |

`get_service_price` returns `0` when the service is absent or inactive. It exists so `voucher` can validate pricing over an interface of primitives only, without the two contracts sharing types.

### Errors

| # | Variant |
| --- | --- |
| 1 | `AlreadyInitialized` |
| 2 | `NotInitialized` |
| 3 | `NotAuthorized` |
| 4 | `ProviderNotFound` |
| 5 | `ProviderExists` |
| 6 | `ProviderNotActive` |
| 7 | `ServiceNotFound` |
| 8 | `InvalidPrice` |
| 9 | `InvalidCountry` |
| 10 | `EmptyName` |
| 11 | `NoPendingAdmin` |

### Events

`provider_registered`, `provider_status_changed`, `service_upserted`, `service_removed`, `attester_added`, `attester_removed`, `admin_changed`, `initialized`

---

## voucher

Escrow and the voucher state machine.

### Types

```rust
pub enum VoucherStatus {
    Funded = 0, Claimed = 1, Attested = 2,
    Settled = 3, Disputed = 4, Refunded = 5,
}

pub struct Voucher {
    pub id: u64,
    pub funder: Address,
    pub beneficiary_ref: BytesN<32>,   // opaque, no PHI
    pub provider: Address,
    pub service_code: u32,
    pub amount: i128,
    pub status: VoucherStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub claimed_at: u64,
    pub attested_at: u64,
    pub dispute_deadline: u64,
}
```

### Functions

| Function | Parameters | Returns | Auth |
| --- | --- | --- | --- |
| `initialize` | `admin, registry, receipt_book, token, dispute_window, fee_bps, fee_account` | `()` | once |
| `create_voucher` | `funder, beneficiary_ref, provider, service_code, amount, expires_at` | `u64` | `funder` |
| `claim` | `provider, voucher_id` | `()` | `provider` |
| `attest` | `attester, voucher_id` | `()` | `attester` |
| `dispute` | `funder, voucher_id, reason_code` | `()` | `funder` |
| `settle` | `voucher_id` | `()` | **none** |
| `refund` | `voucher_id` | `()` | **none** |
| `resolve_dispute` | `admin, voucher_id, refund_funder: bool` | `()` | `admin` |
| `propose_admin` | `admin, new_admin` | `()` | `admin` |
| `accept_admin` | `new_admin` | `()` | `new_admin` |
| `get_voucher` | `voucher_id` | `Voucher` | view |
| `refundable_at` | `voucher_id` | `u64` | view |
| `get_pending_admin` | — | `Option<Address>` | view |
| `get_config` | — | `Config` | view |
| `next_voucher_id` | — | `u64` | view |
| `quote` | `amount` | `(i128, i128)` | view |

`fee_bps` is capped at 1000 (10%) at initialisation.

`settle` and `refund` take no authorisation deliberately: a clinic that has delivered attested care must never depend on the funder or an operator to be paid.

### What each function checks

**`create_voucher`** — amount > 0; expiry in the future; provider is `Active`; the service exists and is active; amount ≥ listed price. Transfers the token *before* writing the voucher, so a failed transfer reverts everything.

**`claim`** — caller is the named provider; provider is still `Active` in the registry; status is `Funded`; not expired.

**`attest`** — caller is a registry attester; caller is **not** the provider; status is `Claimed`. Sets `dispute_deadline = now + dispute_window`.

**`dispute`** — caller is the funder; status is `Claimed`, or `Attested` and before the deadline.

**`settle`** — status is `Attested`; now ≥ `dispute_deadline`. Pays provider, takes fee, mints a receipt.

**`refund`** — status is `Funded` and now ≥ `expires_at`, or status is `Claimed` and now ≥ `expires_at + CLAIM_GRACE_SECS` (7 days). Returns the full amount, no fee. `refundable_at` returns the exact time.

**`propose_admin` / `accept_admin`** — two-step handover, identical in all three contracts. Proposing again replaces the nominee; nothing changes until the nominee signs `accept_admin`.

**`resolve_dispute`** — caller is admin; status is `Disputed`. Routes to funder or provider; no third destination exists.

### Errors

| # | Variant | # | Variant |
| --- | --- | --- | --- |
| 1 | `AlreadyInitialized` | 9 | `InvalidExpiry` |
| 2 | `NotInitialized` | 10 | `InvalidStatus` |
| 3 | `NotAuthorized` | 11 | `VoucherExpired` |
| 4 | `VoucherNotFound` | 12 | `NotYetExpired` |
| 5 | `ProviderNotActive` | 13 | `DisputeWindowOpen` |
| 6 | `ServiceNotOffered` | 14 | `DisputeWindowClosed` |
| 7 | `AmountBelowPrice` | 15 | `InvalidFee` |
| 8 | `InvalidAmount` | 16 | `MathOverflow` |
|  |  | 17 | `NoPendingAdmin` |

### Events

| Event | Topics | Data |
| --- | --- | --- |
| `voucher_created` | `funder`, `provider`, `beneficiary_ref` | `voucher_id`, `service_code`, `amount`, `created_at`, `expires_at` |
| `voucher_claimed` | `provider` | `voucher_id`, `claimed_at`, `service_code`, `amount` |
| `voucher_attested` | `attester` | `voucher_id`, `attested_at`, `dispute_deadline`, `service_code`, `amount` |
| `voucher_disputed` | `funder` | `voucher_id`, `reason_code`, `service_code`, `amount` |
| `voucher_settled` | `provider` | `voucher_id`, `net`, `fee`, `service_code`, `amount` |
| `voucher_refunded` | `funder` | `voucher_id`, `amount`, `service_code` |
| `dispute_resolved` | `admin` | `voucher_id`, `refunded_funder` |

> Fields listed under **Topics** are published in the event's topic list, **not** in its data map. An indexer reading them off the data map gets `undefined`.
>
> Every lifecycle event now carries `service_code` and `amount`, appended **after** the fields each event already had, so one event says what was paid for without a storage lookup. The data map is keyed by field name (an XDR `ScMap`), and existing fields keep their positions: decoders that read by name — including `sci-healthcare-backend` — pick the new fields up without a breaking change. `voucher_refunded` already carried `amount`; it gains `service_code` only.

---

## receipt

A non-transferable record that a care episode was funded and settled. There is deliberately no transfer function.

### Types

```rust
pub struct Receipt {
    pub voucher_id: u64,
    pub beneficiary_ref: BytesN<32>,
    pub provider: Address,
    pub service_code: u32,
    pub amount: i128,
    pub settled_at: u64,
}
```

### Functions

| Function | Parameters | Returns | Auth |
| --- | --- | --- | --- |
| `initialize` | `admin, minter` | `()` | once |
| `set_minter` | `admin, minter` | `()` | `admin` |
| `mint` | `minter, voucher_id, beneficiary_ref, provider, service_code, amount` | `()` | `minter` |
| `get_receipt` | `voucher_id` | `Receipt` | view |
| `count_for` | `beneficiary_ref` | `u32` | view |
| `get_minter` | — | `Address` | view |
| `get_admin` | — | `Address` | view |

Only the minter — the voucher contract — may mint. Because the voucher contract does not exist when the receipt book is initialised, deploy order is: receipt, voucher, then `set_minter`.

### Errors

| # | Variant |
| --- | --- |
| 1 | `AlreadyInitialized` |
| 2 | `NotInitialized` |
| 3 | `NotAuthorized` |
| 4 | `ReceiptNotFound` |
| 5 | `ReceiptExists` |
| 6 | `InvalidAmount` |
| 7 | `NoPendingAdmin` |

### Events

`receipt_minted` — topics `beneficiary_ref`, `provider`; data `voucher_id`, `service_code`, `amount`, `settled_at`

## faucet (testnet only)

Lets anyone trying the demo get test USDC without a server holding the issuer key. The demo token's Stellar Asset Contract names the faucet as its admin, so `drip` can mint. **Never point it at a real asset.** `scripts/deploy-faucet.sh` refuses mainnet.

| Function | Parameters | Returns | Auth |
| --- | --- | --- | --- |
| `initialize` | `admin, token, amount, cooldown` | `()` | once |
| `drip` | `to` | `i128` | `to` |
| `set_drip` | `admin, amount, cooldown` | `()` | `admin` |
| `release_token_admin` | `admin, new_admin` | `()` | `admin` |
| `get_config` | — | `FaucetConfig` | view |
| `next_drip_at` | `to` | `u64` (0 if allowed now) | view |

`drip` requires the recipient's signature, so one wallet cannot drain the faucet into addresses it does not control, and each address may drip once per `cooldown`. A classic account needs a trustline for the token first; the token reports a missing one as its error `#13`.

On testnet the faucet drips 50 USDC once a day: [`CCB25WG7DQ2CSNXPTOHZO2PDZ324FQPZLH3LSNCCLWYM3FEH52GEXC47`](https://stellar.expert/explorer/testnet/contract/CCB25WG7DQ2CSNXPTOHZO2PDZ324FQPZLH3LSNCCLWYM3FEH52GEXC47).

| # | Error |
| --- | --- |
| 1 | `AlreadyInitialized` |
| 2 | `NotInitialized` |
| 3 | `NotAuthorized` |
| 4 | `InvalidAmount` |
| 5 | `CoolingDown` |

---

## Building, releasing and verifying the contracts

### The build

`scripts/build-wasm.sh` is the build. The release workflow, `scripts/deploy.sh`
and `scripts/verify-deployments.sh` all call it, so the wasm that is released,
the wasm that is deployed and the wasm a verifier builds are the same bytes.

```bash
./scripts/build-wasm.sh          # writes dist/*.wasm and dist/SHA256SUMS
```

Four rules make that possible, and every one of them is a trap if it is
ignored:

| Rule | Why |
| --- | --- |
| The toolchain is pinned in `rust-toolchain.toml` (1.96.0) | a different rustc emits different code |
| The release profile is fixed in the workspace `Cargo.toml` (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`) | different flags mean different bytes |
| The **whole workspace** is built in one cargo invocation | cargo unifies features per package, so `cargo build -p sci-registry` links a different `soroban-sdk` than `cargo build` and produces different bytes |
| The cargo registry path is remapped out of the artefact (`--remap-path-prefix`) | the Soroban SDK embeds the absolute path of its own source in its panic messages, so without this the wasm carries the builder's home directory |
| The result is optimised with `wasm-opt -Oz` (binaryen 133) | the release pins the optimiser version, so the optimised bytes do not change under us |

**What "reproducible" covers.** Two builders that use the same toolchain, the
same profile and the same host platform get the same bytes: the same source
built in two different directories, and inside the pinned `rust:1.96.0`
container, both produce identical wasm. It does **not** cover a build on a
different operating system: some dependencies in the tree behave differently
when they are compiled on Windows, which changes the order of a few internal
helper functions in the wasm.

**What is on testnet today.** The contracts on testnet were deployed from an
older commit, built on the maintainer's Windows machine before this recipe
existed. They carry that machine's paths, so a fresh build does not reproduce
their bytes even though it exposes exactly the same interface. The voucher is
further behind: its lifecycle events do not carry `service_code` and `amount`
yet, so the deployed voucher is not the one in this repository. Redeploying
with `scripts/deploy.sh` fixes both.

### The release

Pushing a `v*` tag runs `.github/workflows/release.yml`, which tests, builds
and attaches to the GitHub release:

- `sci_registry.wasm`, `sci_voucher.wasm`, `sci_receipt.wasm`, `sci_faucet.wasm` — the optimised contracts,
- `SHA256SUMS` — the SHA-256 of every file,
- release notes that explain how to compare the hash with a deployed contract.

Comparing a deployed contract with a release is one command:

```bash
stellar contract info hash --contract-id <CONTRACT_ID> --network testnet
grep sci_voucher SHA256SUMS
```

The two hashes must be identical.

### The check

```bash
./scripts/verify-deployments.sh testnet
```

For every contract in `deployments/testnet.env` the script fetches the
deployed wasm with `stellar contract fetch` and compares it with the local
build twice:

- **hash** — the SHA-256 of the whole file. Equal hashes mean the deployed
  bytes are the bytes this source produces.
- **interface** — the contract interface (functions, types, errors and events)
  as the Stellar host reads it, through `stellar contract info interface`.
  Equal interfaces mean the deployed contract behaves like the source even when
  the bytes differ.

The script exits non-zero when either differs. `--wasm-dir DIR` compares
against wasm files you already have (the artefacts from a release, for
example) instead of building; `--skip-build` reuses what is already in that
directory.

### Coverage

CI measures coverage with `cargo llvm-cov` and fails when it drops. The
contract sources are at **98.14% of lines** and **95.99% of regions**; the
thresholds in `.github/workflows/ci.yml` sit just under, at 98% and 95%. Test
files are excluded from the measurement: the number that matters is how much of
the contract code the tests reach. Raise the threshold in the same pull
request that adds the tests.

### Dependencies

`cargo deny check` runs in CI against the policy in `deny.toml`: the RustSec
advisory database, the licence allow list, the duplicate and wildcard bans and
the source allow list. There are no exceptions today. If one is needed, it has
to carry a reason:

```toml
[advisories]
ignore = [{ id = "RUSTSEC-0000-0000", reason = "why this does not apply" }]
```

Run it locally with:

```bash
cargo install --locked cargo-deny --version 0.20.2
cargo deny check
```
