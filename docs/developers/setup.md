---
title: Local setup
nav_order: 9
---

# Local setup

Three repositories. Build them in this order — the app needs contract addresses that only exist after deployment.

| Repo | What it is |
| --- | --- |
| [sci-healthcare-contracts](https://github.com/Godgift-care/sci-healthcare-contracts) | Soroban contracts (Rust) |
| [sci-healthcare-backend](https://github.com/Godgift-care/sci-healthcare-backend) | Event indexer and read API |
| [sci-healthcare-frontend](https://github.com/Godgift-care/sci-healthcare-frontend) | Web app |

## Prerequisites

| Tool | Version |
| --- | --- |
| Rust | 1.96.0 |
| wasm target | `wasm32v1-none` |
| Stellar CLI | 27.0.0+ |
| wasm-opt (binaryen) | 133 |
| Node | 22 LTS |

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.96.0
rustup target add wasm32v1-none
cargo install --locked stellar-cli
cargo install --locked wasm-opt   # or download binaryen 133 from GitHub releases
```

## 1. Contracts

```bash
git clone https://github.com/Godgift-care/sci-healthcare-contracts
cd sci-healthcare-contracts

cargo test                                    # 135 tests
cargo build --target wasm32v1-none --release
./scripts/build-wasm.sh                       # the release build: optimised wasm + SHA256SUMS
```

`scripts/build-wasm.sh` is the build the release workflow and the deployment
use, so the wasm you build is the wasm that is released and deployed. See
[Contract reference](contracts.md#building-releasing-and-verifying-the-contracts)
for the rules it follows.

Create and fund identities, then deploy:

```bash
for id in sci-admin sci-issuer sci-clinic sci-attester sci-funder; do
  stellar keys generate --network testnet --fund $id
done

./scripts/deploy.sh testnet --dry-run   # preview every command; sends nothing
./scripts/deploy.sh testnet             # writes deployments/testnet.env
./scripts/seed.sh testnet
```

`deploy.sh` deploys in dependency order — registry and receipt first, then voucher — initialises each, points the receipt book at the voucher contract, and writes every address to `deployments/testnet.env`. Re-running it refuses to overwrite an existing env file unless you pass `--force`; use `--dry-run` to preview without building, deploying or sending a transaction. `scripts/test-deploy.sh` covers both behaviours.

`seed.sh` creates a demo clinic with three services, authorises an attester, and funds two vouchers.

The default dispute window is 72 hours, so the seeded voucher 1 stays `Attested`
for three days before it can settle. That is the state the end-to-end check
asserts on.

### The end-to-end check on a real node

`cargo test` runs everything in an in-process host. It never touches the CLI,
the RPC or the shell scripts, so it cannot notice that `deploy.sh` started
passing a flag the CLI no longer accepts. `scripts/e2e-local.sh` closes that gap
by running the real thing against a standalone node:

```bash
docker run --rm -d -p 8000:8000 --name stellar stellar/quickstart:latest --local
./scripts/e2e-local.sh
```

The script waits for the node, registers it with the CLI as `local`, creates
and funds the five identities `deploy.sh` and `seed.sh` expect, runs both
scripts, and then reads voucher 1 back from the chain and asserts it is
`Attested`. It prints a banner before every step and names the step that failed,
so a red job says where it stopped.

Addresses land in `deployments/local.env` (gitignored). Re-running the script
against the same node deploys again with `--force`; stop and remove the
container to start from a clean ledger:

```bash
docker stop stellar
```

`.github/workflows/e2e-local.yml` runs exactly this on every push to `main`,
because pulling the image and building the release wasm makes it the slowest
job here. It is not part of the pull request checks.

### The testnet dispute window

`deploy.sh` defaults to a **72-hour** dispute window. That makes a demo impossible to finish, so deploy with a short one for local work:

```bash
DISPUTE_WINDOW=60 ./scripts/deploy.sh testnet
```

## 2. Backend

```bash
git clone https://github.com/Godgift-care/sci-healthcare-backend
cd sci-healthcare-backend
npm install
cp .env.example .env
```

Copy the addresses from `deployments/testnet.env` into `.env`, then:

```bash
npx prisma generate
npx prisma db push
npm run dev          # http://localhost:8080
```

> **Set `INDEXER_START_LEDGER`.** Left at `0` the indexer starts at the current tip and silently skips everything that happened before, so your seeded data never appears. Set it to a ledger just before you deployed:
>
> ```bash
> curl -s -X POST https://soroban-testnet.stellar.org \
>   -H 'Content-Type: application/json' \
>   -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}'
> ```

Check it caught up:

```bash
curl -s localhost:8080/stats | jq
curl -s localhost:8080/ready | jq
```

## 3. Frontend

```bash
git clone https://github.com/Godgift-care/sci-healthcare-frontend
cd sci-healthcare-frontend
npm install
cp .env.example .env.local
```

Fill in the same addresses with the `NEXT_PUBLIC_` prefix, then:

```bash
npm run dev          # http://localhost:3000
```

## Things that go wrong

**`DATABASE_URL` creates `prisma/prisma/dev.db`.** Prisma resolves a relative SQLite path against the schema file, not the project root. Use `file:./dev.db`, not `file:./prisma/dev.db`.

**The app still calls localhost after deploying.** `NEXT_PUBLIC_*` is inlined at build time. Setting it in the host's runtime environment does nothing — it has to be in the build environment, and you have to rebuild.

**Indexed rows contain the string `"undefined"`.** A contract event field marked `#[topic]` is published in the topic list, not the data map. Reading it off the data map yields `undefined`, which stringifies. Check `TOPIC_FIELDS` in `src/stellar/events.ts` matches the contract.

**`Error(Contract, #13)` when settling.** The dispute window is still open. Wait it out, or redeploy with a shorter `DISPUTE_WINDOW`.

**Enum arguments rejected by the CLI.** Unit enums with explicit discriminants take the integer, not the name: `--status 1`, not `--status Active`.

**`BytesN` arguments rejected.** These take raw hex, not JSON. Pass the 64 hex characters directly.
