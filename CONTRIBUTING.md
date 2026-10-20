# Contributing to SCI Healthcare Contracts

Thanks for helping. This repo holds the Soroban contracts, so changes here move
real escrowed value — the bar is deliberately high.

## Before you start

1. Comment on the issue you want and wait to be assigned, so two people do not
   build the same thing.
2. If you are proposing a design change rather than fixing a defect, open an
   issue first and agree the approach before writing code.

## Setup

```bash
rustup toolchain install 1.96.0
rustup target add wasm32v1-none
cargo install --locked stellar-cli
cargo test
```

The toolchain is pinned in `rust-toolchain.toml`. Please do not change it in a
feature PR.

### Dev container

The repository ships a [dev container](.devcontainer/) with everything pinned
to the versions CI uses: Rust 1.96.0 with the `wasm32v1-none` target, the
Stellar CLI 27.1.0, `wasm-opt` from binaryen 133 and `just`. Open the
repository in VS Code with the Dev Containers extension (**Reopen in
Container**) or in GitHub Codespaces, and `cargo test` and `just wasm` work
straight away. When CI bumps one of those versions, bump it in
`.devcontainer/Dockerfile` in the same change.

### Common tasks

A [`justfile`](justfile) wraps the commands CI and the docs use, so there is
one entry point. Install [`just`](https://github.com/casey/just) (it is already
in the dev container), then:

| Recipe | Runs |
| --- | --- |
| `just` | lists the recipes |
| `just build` | `cargo build --all` |
| `just test` | `cargo test --all` |
| `just fmt` / `just fmt-check` | `cargo fmt --all` / with `-- --check` |
| `just clippy` | `cargo clippy --all-targets --all-features -- -D warnings` |
| `just check` | `fmt-check`, `clippy` and `test` — what CI runs on a pull request |
| `just wasm [out]` | `scripts/build-wasm.sh` (default `dist`) |
| `just deploy [network] [flags]` | `scripts/deploy.sh`, e.g. `just deploy testnet --dry-run` |
| `just seed [network]` | `scripts/seed.sh` |
| `just verify [network]` | `scripts/verify-deployments.sh` |
| `just test-deploy` | `scripts/test-deploy.sh` |
| `just e2e-local` | `scripts/e2e-local.sh` against a local node on port 8000 |

The network defaults to `testnet`; set `NETWORK` to change it.

## Non-negotiables

These are the rules a review will fail you on.

- **No patient health information on chain, ever.** Service codes stay coarse
  categories. `beneficiary_ref` stays an opaque 32 bytes. If a change would let
  anyone infer a diagnosis, a result, or an identity from ledger data, it will
  be rejected regardless of how useful it is.
- **No `unwrap()` or `expect()` outside `#[cfg(test)]`.** Return a
  `#[contracterror]` variant instead.
- **No floating point.** Money is `i128` in the token's smallest unit. Fees are
  basis points with integer division, and truncation must favour the provider,
  never the protocol.
- **Use checked arithmetic** on anything that could overflow. `MathOverflow`
  exists for this.
- **Extend TTL on every persistent read and write.** An entry that expires is an
  entry that has silently lost user funds.
- **`require_auth()` on the acting address**, and check role membership
  separately. Authenticating is not the same as being authorised.
- **Every new function needs tests for the failure paths**, not just the happy
  one. Look at how existing tests assert specific error variants.

## Commits

Conventional commits, one logical change per commit:

```
feat(voucher): add partial refund for over-funded vouchers
fix(registry): reject a two-character country code containing digits
test(receipt): cover minting after a minter rotation
docs(readme): correct the deploy order
```

Types: `feat`, `fix`, `test`, `docs`, `refactor`, `chore`, `perf`.

## Pull requests

- Branch from `main`, named `feat/short-description` or `fix/short-description`.
- `cargo test` and `cargo build --target wasm32v1-none --release` must both pass.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` must be clean.
- Describe **what breaks if this is wrong**, not just what you changed.
- If you changed an event's fields, say so explicitly — the indexer in
  `sci-healthcare-backend` decodes events by name, and moving a field between
  topic and data is a breaking change that needs a matching PR there.

## Adding a contract event

Events are part of the public interface. When adding or changing one:

1. Define it with `#[contractevent]` in the contract's `events.rs`.
2. Mark fields `#[topic]` only if they should be indexable. Topic fields are
   published in the topic list, **not** in the data map.
3. Update `TOPIC_FIELDS` in `sci-healthcare-backend/src/stellar/events.ts` in the
   same change set, and open a linked PR there.

## How pull requests are merged

A bot merges pull requests automatically (squash) as soon as every CI check on the latest commit has passed, provided the pull request:

- is not a draft and has no `do-not-merge` label,
- links the issue you were assigned on Drips Wave, with a line such as `Closes #123` in the description (Wave points are awarded when that issue is closed as completed, which the bot does on merge),
- has no merge conflicts and is up to date with `main` (the bot comments if it is behind),
- passes the security scan (the `security-scan` status, see below),
- and, if it changes anything under `.github/` (CI workflows and repository automation), has been approved by a maintainer. The bot comments with the files that need approval.

So open your pull request as a draft until it is ready, and keep CI green. To stop an automatic merge, add the `do-not-merge` label or convert the pull request back to a draft.

### Security scan

Every pull request is scanned for malicious changes before it can be merged. The scan runs from `main`, reads your changes as data and never runs them. It reports on the pull request as the `security-scan` status and, if anything needs attention, in a comment listing each finding.

- **block** findings are never merged automatically. Examples are executables and unexpected binaries, hidden or bidirectional Unicode characters (Trojan Source), crypto miners, reverse shells, piping a download into a shell, reading credential files, dependencies from git URLs, tarballs or paths outside the repository, look-alike (typosquatted) package names, packages that do not exist on npm, lockfile entries from another registry or without an integrity hash, crates from outside crates.io, and packages listed as malware in the OSV database.
- **review** findings are common in legitimate work but also how malware gets in. Examples are new dependencies that are recent or rarely used, changed npm scripts, `eval` and spawned processes, encoded or minified code, new network hosts or Stellar addresses, shell scripts, and security-sensitive files. A maintainer reads the change and adds the `security-reviewed` label, after which it merges as usual. A new push removes the label, so the latest commit is always the one reviewed.

If a finding is a false positive, say so on the pull request; please do not try to reword code to get past the scan.

## Wave program

Issues carry a complexity label that maps to Wave points: `trivial` (100),
`medium` (150), `high` (200). Please read the
[Wave rules](https://docs.drips.network/wave/terms-and-rules/) — in particular,
low-effort or unreviewed LLM-generated submissions are explicitly disallowed and
will be closed. Understand what you submit and test it.
