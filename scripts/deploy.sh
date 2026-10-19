#!/usr/bin/env bash
#
# Deploys the SCI Healthcare contracts in dependency order and wires them
# together. Prints a copy-pasteable environment block at the end.
#
# Usage:  ./scripts/deploy.sh [network]     (default: testnet)
#
# Requires: stellar CLI >= 27, wasm-opt (binaryen 133), a funded source
# identity named $ADMIN.
#
# The contracts are built by scripts/build-wasm.sh, the same script the
# release workflow uses, so what is deployed is what is released and
# scripts/verify-deployments.sh can check it.

set -euo pipefail

NETWORK="${1:-testnet}"
ADMIN="${ADMIN:-sci-admin}"
ISSUER="${ISSUER:-sci-issuer}"
DISPUTE_WINDOW="${DISPUTE_WINDOW:-259200}"   # 72 hours, in seconds
FEE_BPS="${FEE_BPS:-100}"                    # 1%

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_DIR="$ROOT/dist"
OUT="$ROOT/deployments/$NETWORK.env"
mkdir -p "$(dirname "$OUT")"

say() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

ADMIN_ADDR="$(stellar keys address "$ADMIN")"
ISSUER_ADDR="$(stellar keys address "$ISSUER")"

say "Building contracts"
"$ROOT/scripts/build-wasm.sh" "$WASM_DIR" >/dev/null
cat "$WASM_DIR/SHA256SUMS"

say "Deploying settlement token (USDC test asset)"
stellar contract asset deploy \
  --asset "USDC:$ISSUER_ADDR" \
  --source-account "$ISSUER" \
  --network "$NETWORK" >/dev/null 2>&1 || echo "  (already deployed)"
USDC_ID="$(stellar contract id asset --asset "USDC:$ISSUER_ADDR" --network "$NETWORK")"
echo "  USDC: $USDC_ID"

# Contracts deploy in dependency order: registry and receipt own no
# references, voucher points at both.
say "Deploying registry"
REGISTRY_ID="$(stellar contract deploy \
  --wasm "$WASM_DIR/sci_registry.wasm" \
  --source-account "$ADMIN" --network "$NETWORK")"
echo "  registry: $REGISTRY_ID"

say "Deploying receipt book"
RECEIPT_ID="$(stellar contract deploy \
  --wasm "$WASM_DIR/sci_receipt.wasm" \
  --source-account "$ADMIN" --network "$NETWORK")"
echo "  receipt:  $RECEIPT_ID"

say "Deploying voucher escrow"
VOUCHER_ID="$(stellar contract deploy \
  --wasm "$WASM_DIR/sci_voucher.wasm" \
  --source-account "$ADMIN" --network "$NETWORK")"
echo "  voucher:  $VOUCHER_ID"

say "Initializing registry"
stellar contract invoke --id "$REGISTRY_ID" --source-account "$ADMIN" \
  --network "$NETWORK" -- initialize --admin "$ADMIN_ADDR"

# The receipt book needs a minter at init but the voucher contract does not
# exist yet at that point, so the admin stands in and is repointed below.
say "Initializing receipt book"
stellar contract invoke --id "$RECEIPT_ID" --source-account "$ADMIN" \
  --network "$NETWORK" -- initialize --admin "$ADMIN_ADDR" --minter "$ADMIN_ADDR"

say "Initializing voucher escrow"
stellar contract invoke --id "$VOUCHER_ID" --source-account "$ADMIN" \
  --network "$NETWORK" -- initialize \
  --admin "$ADMIN_ADDR" \
  --registry "$REGISTRY_ID" \
  --receipt_book "$RECEIPT_ID" \
  --token "$USDC_ID" \
  --dispute_window "$DISPUTE_WINDOW" \
  --fee_bps "$FEE_BPS" \
  --fee_account "$ADMIN_ADDR"

say "Pointing receipt book at the voucher contract"
stellar contract invoke --id "$RECEIPT_ID" --source-account "$ADMIN" \
  --network "$NETWORK" -- set_minter --admin "$ADMIN_ADDR" --minter "$VOUCHER_ID"

cat > "$OUT" <<ENVEOF
STELLAR_NETWORK=$NETWORK
REGISTRY_CONTRACT_ID=$REGISTRY_ID
VOUCHER_CONTRACT_ID=$VOUCHER_ID
RECEIPT_CONTRACT_ID=$RECEIPT_ID
USDC_CONTRACT_ID=$USDC_ID
USDC_ISSUER=$ISSUER_ADDR
ADMIN_ADDRESS=$ADMIN_ADDR
ENVEOF

say "Deployed. Copy into your backend and frontend .env files:"
cat <<BLOCK

  REGISTRY_CONTRACT_ID=$REGISTRY_ID
  VOUCHER_CONTRACT_ID=$VOUCHER_ID
  RECEIPT_CONTRACT_ID=$RECEIPT_ID
  USDC_CONTRACT_ID=$USDC_ID

Saved to $OUT
BLOCK
