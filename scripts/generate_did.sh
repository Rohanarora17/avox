#!/bin/bash
# scripts/generate_did.sh - Generate Candid .did file for avox_backend canister

set -e

CANISTER=avox_backend
DID_PATH=src/avox_backend/${CANISTER}.did
WASM_PATH=target/wasm32-unknown-unknown/release/${CANISTER}.wasm

# Build the canister to ensure candid export is up to date
cargo build --target wasm32-unknown-unknown --package $CANISTER

# Use candid-extractor to generate the .did file
if command -v candid-extractor &> /dev/null; then
  echo "Generating .did file using candid-extractor..."
  candid-extractor $WASM_PATH > $DID_PATH
  echo "Candid file written to $DID_PATH"
else
  echo "candid-extractor not found. Please install it (https://github.com/dfinity/candid/tree/master/tools/extractor) to generate the .did file."
  exit 1
fi 