#!/bin/bash

# deploy_icrc1_ledger.sh - Deploys the ICRC-1 ledger canister with recommended init args

set -e

# Get principal for minting and initial balances
PRINCIPAL=$(dfx identity get-principal)

TOKEN_SYMBOL="AVOX"
TOKEN_NAME="Avox"
TRANSFER_FEE=10000
DECIMALS=8
PRE_MINTED_TOKENS=1000000000000
ARCHIVE_CYCLES=10000000000000
NUM_BLOCKS_TO_ARCHIVE=1000
TRIGGER_THRESHOLD=2000

# Prepare the init arg
INIT_ARG="(variant { Init = record {
  decimals = opt ($DECIMALS : nat8);
  token_symbol = \"$TOKEN_SYMBOL\";
  transfer_fee = $TRANSFER_FEE : nat;
  metadata = vec {};
  minting_account = record {
    owner = principal \"$PRINCIPAL\";
    subaccount = null;
  };
  initial_balances = vec {
    record {
      record {
        owner = principal \"$PRINCIPAL\";
        subaccount = null;
      };
      $PRE_MINTED_TOKENS : nat;
    }
  };
  fee_collector_account = null;
  archive_options = record {
    num_blocks_to_archive = $NUM_BLOCKS_TO_ARCHIVE : nat64;
    max_transactions_per_response = null;
    trigger_threshold = $TRIGGER_THRESHOLD : nat64;
    more_controller_ids = null;
    max_message_size_bytes = null;
    cycles_for_archive_creation = opt ($ARCHIVE_CYCLES : nat64);
    node_max_memory_size_bytes = null;
    controller_id = principal \"$PRINCIPAL\";
  };
  max_memo_length = null;
  token_name = \"$TOKEN_NAME\";
  feature_flags = opt record { icrc2 = true };
} })"

# Create the canister if it doesn't exist
dfx canister status icrc1_ledger 2>/dev/null || dfx canister create icrc1_ledger

echo "Deploying icrc1_ledger with principal: $PRINCIPAL"
dfx deploy icrc1_ledger --argument "$INIT_ARG"

echo "ICRC-1 ledger deployed. Canister ID: $(dfx canister id icrc1_ledger)" 