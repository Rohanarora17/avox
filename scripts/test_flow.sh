#!/bin/bash

# test_flow.sh - Automated test flow for Avox backend (full lifecycle, two identities, balances, profiles)

set -e

# Colors for demo readability
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BACKEND_ID=$(dfx canister id avox_backend)
LEDGER_ID=$(dfx canister id icrc1_ledger)
ORIGINAL_IDENTITY=$(dfx identity whoami)
PRIZE_AMOUNT=100000000
FEE=10000
TOTAL_AMOUNT=$((PRIZE_AMOUNT + FEE))
TITLE="Automated Test Bounty"
DESCRIPTION="Automated test description"
GITHUB_URL="https://github.com/testuser/repo/issues/42"
PR_URL="https://github.com/testuser/repo/pull/99"
COMMENT="Automated test solution."
PARTICIPANT_IDENTITY="participant-test"

# Helper: Convert hex to Candid vec nat8
hex_to_vecnat8() {
  local hex="$1"
  local out="opt vec {"
  for ((i=0; i<${#hex}; i+=2)); do
    byte="0x${hex:$i:2}"
    out="$out $((byte))"
    if [ $i -lt 62 ]; then
      out="$out;"
    fi
  done
  out="$out }"
  echo "$out"
}

# Helper: Print balance for a principal/subaccount
print_balance() {
  local owner="$1"
  local subaccount="$2"
  if [ -z "$subaccount" ]; then
    echo -e "${YELLOW}Balance for $owner (main):${NC}"
    dfx canister call $LEDGER_ID icrc1_balance_of "(record { owner = principal \"$owner\"; subaccount = null })"
  else
    local vecnat8=$(hex_to_vecnat8 "$subaccount")
    echo -e "${YELLOW}Balance for $owner subaccount $subaccount:${NC}"
    dfx canister call $LEDGER_ID icrc1_balance_of "(record { owner = principal \"$owner\"; subaccount = $vecnat8 })"
  fi
}

# 1. Update and query user profile
PRINCIPAL=$(dfx identity get-principal)
echo -e "\n${BLUE}========== [1] Update and Query User Profile ==========${NC}"
echo -e "${GREEN}Updating user profile for creator...${NC}"
dfx canister call $BACKEND_ID update_user_profile '(record {
  name = opt "Test User";
  github = opt "https://github.com/testuser";
  twitter = opt "https://twitter.com/testuser";
  pfp_url = opt "https://example.com/pfp.png";
})'
echo -e "${GREEN}Querying user profile for creator...${NC}"
dfx canister call $BACKEND_ID get_user_profile "(principal \"$PRINCIPAL\")"

# 2. Create bounty as creator
echo -e "\n${BLUE}========== [2] Create and Fund Bounty ==========${NC}"
echo -e "${GREEN}Creating bounty as $ORIGINAL_IDENTITY...${NC}"
BOUNTY_RESULT=$(dfx canister call $BACKEND_ID create_bounty "(record {
  title = \"$TITLE\";
  description = \"$DESCRIPTION\";
  github_issue_url = \"$GITHUB_URL\";
  prize_amount = $PRIZE_AMOUNT;
  token_ledger = principal \"$LEDGER_ID\";
  from_subaccount = null;
  fee = null;
})")
echo "$BOUNTY_RESULT"
BOUNTY_ID=$(echo "$BOUNTY_RESULT" | awk '/Ok = record/ {getline; match($0, /[0-9]+/); print substr($0, RSTART, RLENGTH); exit}')
ESCROW_ACCOUNT=$(echo "$BOUNTY_RESULT" | awk -F'"' '/Ok = record/ {getline; getline; print $2; exit}')
ESCROW_SUBACCOUNT=$(echo "$ESCROW_ACCOUNT" | grep -oE '[a-f0-9]{64}$')
VECNAT8_SUBACCOUNT=$(hex_to_vecnat8 "$ESCROW_SUBACCOUNT")
echo -e "${YELLOW}Bounty ID: $BOUNTY_ID${NC}"
echo -e "${YELLOW}Escrow Subaccount: $ESCROW_SUBACCOUNT${NC}"

# 3. Fund escrow
echo -e "${GREEN}Funding escrow account with $TOTAL_AMOUNT tokens (prize + fee)...${NC}"
TRANSFER_ARG_FILE=$(mktemp)
cat > "$TRANSFER_ARG_FILE" <<EOF
(record {
  to = record {
    owner = principal "$BACKEND_ID";
    subaccount = $VECNAT8_SUBACCOUNT;
  };
  amount = $TOTAL_AMOUNT;
  fee = null;
  memo = null;
  from_subaccount = null;
  created_at_time = null;
})
EOF
dfx canister call $LEDGER_ID icrc1_transfer --argument-file "$TRANSFER_ARG_FILE"
rm "$TRANSFER_ARG_FILE"

print_balance "$BACKEND_ID" "$ESCROW_SUBACCOUNT"

echo -e "${GREEN}Verifying escrow deposit (should be true)...${NC}"
dfx canister call $BACKEND_ID verify_escrow_deposit "($BOUNTY_ID)"

echo -e "${GREEN}Fetching bounty details...${NC}"
dfx canister call $BACKEND_ID get_bounty "($BOUNTY_ID)"

echo -e "${GREEN}Listing all active bounties...${NC}"
dfx canister call $BACKEND_ID get_active_bounties

echo -e "${GREEN}Getting escrow account for bounty...${NC}"
dfx canister call $BACKEND_ID get_escrow_account "($BOUNTY_ID)"

# 4. Create participant identity if needed
echo -e "\n${BLUE}========== [3] Participant Submits Solution ==========${NC}"
echo -e "${GREEN}Checking for participant identity...${NC}"
if ! dfx identity list | grep -q "^$PARTICIPANT_IDENTITY$"; then
  echo -e "${YELLOW}Creating new identity: $PARTICIPANT_IDENTITY${NC}"
  dfx identity new $PARTICIPANT_IDENTITY
fi
PARTICIPANT_PRINCIPAL=$(dfx identity --identity $PARTICIPANT_IDENTITY get-principal)
echo -e "${YELLOW}Participant principal: $PARTICIPANT_PRINCIPAL${NC}"

print_balance "$PARTICIPANT_PRINCIPAL" ""

echo -e "${GREEN}Switching to participant and submitting solution...${NC}"
dfx identity use $PARTICIPANT_IDENTITY
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $BOUNTY_ID; pr_url = \"$PR_URL\"; comment = \"$COMMENT\" })"

dfx identity use $ORIGINAL_IDENTITY

echo -e "${GREEN}Listing submissions for bounty...${NC}"
dfx canister call $BACKEND_ID get_bounty "($BOUNTY_ID)"

echo -e "${GREEN}Selecting winner as creator...${NC}"
dfx canister call $BACKEND_ID select_winner "($BOUNTY_ID, principal \"$PARTICIPANT_PRINCIPAL\")"

echo -e "\n${BLUE}========== [4] Winner Claims Reward ==========${NC}"
echo -e "${GREEN}Switching to participant and claiming reward...${NC}"
dfx identity use $PARTICIPANT_IDENTITY
dfx canister call $BACKEND_ID claim_reward "($BOUNTY_ID)"

print_balance "$PARTICIPANT_PRINCIPAL" ""

dfx identity use $ORIGINAL_IDENTITY

echo -e "${GREEN}Try to claim reward again (should fail, expected)...${NC}"
if dfx canister call $BACKEND_ID claim_reward "($BOUNTY_ID)"; then
  echo -e "${RED}ERROR: Double claim succeeded (should not happen)!${NC}"
else
  echo -e "${YELLOW}EXPECTED FAILURE: Double claim prevented.${NC}"
fi

echo -e "${GREEN}Try to cancel bounty after submission (should fail, expected)...${NC}"
if dfx canister call $BACKEND_ID cancel_bounty "($BOUNTY_ID)"; then
  echo -e "${RED}ERROR: Cancel after submission succeeded (should not happen)!${NC}"
else
  echo -e "${YELLOW}EXPECTED FAILURE: Cancel after submission prevented.${NC}"
fi

echo -e "\n${BLUE}========== [5] User and Bounty Queries ==========${NC}"
echo -e "${GREEN}Get my created bounties...${NC}"
dfx canister call $BACKEND_ID get_user_created_bounties "(principal \"$PRINCIPAL\")"

echo -e "${GREEN}Get my submissions...${NC}"
dfx canister call $BACKEND_ID get_user_submissions "(principal \"$PRINCIPAL\")"

echo -e "${GREEN}Get participant submissions...${NC}"
dfx canister call $BACKEND_ID get_user_submissions "(principal \"$PARTICIPANT_PRINCIPAL\")"

echo -e "${GREEN}Check token balance for creator...${NC}"
print_balance "$PRINCIPAL" ""

echo -e "${GREEN}Check token balance for participant...${NC}"
print_balance "$PARTICIPANT_PRINCIPAL" ""

echo -e "${GREEN}Final bounty state:${NC}"
dfx canister call $BACKEND_ID get_bounty "($BOUNTY_ID)"

echo -e "${GREEN}Query user profile after actions...${NC}"
dfx canister call $BACKEND_ID get_user_profile "(principal \"$PRINCIPAL\")"

echo -e "${GREEN}Query participant profile...${NC}"
dfx canister call $BACKEND_ID get_user_profile "(principal \"$PARTICIPANT_PRINCIPAL\")"

echo -e "${GREEN}Get all bounties (including inactive)...${NC}"
dfx canister call $BACKEND_ID get_all_bounties

echo -e "${GREEN}Get escrow account for bounty (again)...${NC}"
dfx canister call $BACKEND_ID get_escrow_account "($BOUNTY_ID)"

echo -e "${GREEN}Greet endpoint (should return greeting)...${NC}"
dfx canister call $BACKEND_ID greet '("Avox Test")'

echo -e "\n${BLUE}========== [5.1] Pagination and Filtering Tests ==========${NC}"
echo -e "${GREEN}Get first bounty with pagination (offset=0, limit=1)...${NC}"
dfx canister call $BACKEND_ID get_bounties_paginated "(0, 1)"
echo -e "${GREEN}Get second bounty with pagination (offset=1, limit=1)...${NC}"
dfx canister call $BACKEND_ID get_bounties_paginated "(1, 1)"
echo -e "${GREEN}Get all Active bounties (paginated, offset=0, limit=10)...${NC}"
dfx canister call $BACKEND_ID get_bounties_by_status "(variant { Active }, 0, 10)"
echo -e "${GREEN}Get all Completed bounties (paginated, offset=0, limit=10)...${NC}"
dfx canister call $BACKEND_ID get_bounties_by_status "(variant { Completed }, 0, 10)"

echo -e "\n${BLUE}========== [6] Multi-Bounty, Multi-Participant, and Negative Tests ==========${NC}"
echo "[MULTI] Creating a second bounty as $ORIGINAL_IDENTITY..."
BOUNTY_RESULT2=$(dfx canister call $BACKEND_ID create_bounty "(record {
  title = \"Second Test Bounty\";
  description = \"Second bounty for multi test\";
  github_issue_url = \"https://github.com/testuser/repo/issues/43\";
  prize_amount = $PRIZE_AMOUNT;
  token_ledger = principal \"$LEDGER_ID\";
  from_subaccount = null;
  fee = null;
})")
BOUNTY_ID2=$(echo "$BOUNTY_RESULT2" | awk '/Ok = record/ {getline; match($0, /[0-9]+/); print substr($0, RSTART, RLENGTH); exit}')
ESCROW_ACCOUNT2=$(echo "$BOUNTY_RESULT2" | awk -F'"' '/Ok = record/ {getline; getline; print $2; exit}')
ESCROW_SUBACCOUNT2=$(echo "$ESCROW_ACCOUNT2" | grep -oE '[a-f0-9]{64}$')
VECNAT8_SUBACCOUNT2=$(hex_to_vecnat8 "$ESCROW_SUBACCOUNT2")

TRANSFER_ARG_FILE2=$(mktemp)
cat > "$TRANSFER_ARG_FILE2" <<EOF
(record {
  to = record {
    owner = principal "$BACKEND_ID";
    subaccount = $VECNAT8_SUBACCOUNT2;
  };
  amount = $TOTAL_AMOUNT;
  fee = null;
  memo = null;
  from_subaccount = null;
  created_at_time = null;
})
EOF
echo "[MULTI] Funding second bounty escrow..."
dfx canister call $LEDGER_ID icrc1_transfer --argument-file "$TRANSFER_ARG_FILE2"
rm "$TRANSFER_ARG_FILE2"
dfx canister call $BACKEND_ID verify_escrow_deposit "($BOUNTY_ID2)"

# Create a second participant
PARTICIPANT2_IDENTITY="participant2-test"
if ! dfx identity list | grep -q "^$PARTICIPANT2_IDENTITY$"; then
  echo "Creating new identity: $PARTICIPANT2_IDENTITY"
  dfx identity new $PARTICIPANT2_IDENTITY
fi
PARTICIPANT2_PRINCIPAL=$(dfx identity --identity $PARTICIPANT2_IDENTITY get-principal)
echo "Second participant principal: $PARTICIPANT2_PRINCIPAL"

# Both participants submit to the second bounty
dfx identity use $PARTICIPANT_IDENTITY
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $BOUNTY_ID2; pr_url = \"https://github.com/testuser/repo/pull/201\"; comment = \"First participant solution.\" })"
dfx identity use $PARTICIPANT2_IDENTITY
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $BOUNTY_ID2; pr_url = \"https://github.com/testuser/repo/pull/202\"; comment = \"Second participant solution.\" })"

# Negative: participant2 tries to select winner (should fail)
echo "[NEGATIVE] Participant2 tries to select winner (should fail):"
dfx canister call $BACKEND_ID select_winner "($BOUNTY_ID2, principal \"$PARTICIPANT2_PRINCIPAL\")" || true

# Switch to creator and select participant2 as winner
dfx identity use $ORIGINAL_IDENTITY
dfx canister call $BACKEND_ID select_winner "($BOUNTY_ID2, principal \"$PARTICIPANT2_PRINCIPAL\")"

# Negative: participant1 tries to claim reward (should fail)
dfx identity use $PARTICIPANT_IDENTITY
echo "[NEGATIVE] Participant1 tries to claim reward (should fail):"
dfx canister call $BACKEND_ID claim_reward "($BOUNTY_ID2)" || true

# Participant2 claims reward (should succeed)
dfx identity use $PARTICIPANT2_IDENTITY
dfx canister call $BACKEND_ID claim_reward "($BOUNTY_ID2)"

# Negative: participant2 tries to submit again to the same bounty (should fail)
echo "[NEGATIVE] Participant2 tries to submit again (should fail):"
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $BOUNTY_ID2; pr_url = \"https://github.com/testuser/repo/pull/203\"; comment = \"Duplicate submission.\" })" || true

# Negative: try to submit to a non-active bounty (e.g., after winner selected)
echo "[NEGATIVE] Participant1 tries to submit to completed bounty (should fail):"
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $BOUNTY_ID2; pr_url = \"https://github.com/testuser/repo/pull/204\"; comment = \"Late submission.\" })" || true

# Switch back to original identity
dfx identity use $ORIGINAL_IDENTITY

echo -e "\n${GREEN}All multi-bounty, multi-participant, and negative tests completed.${NC}\n"

# 7. Deadline Expiry and Refund Test
DEADLINE_BOUNTY_TITLE="Deadline Test Bounty"
DEADLINE_BOUNTY_DESC="Bounty with short deadline for expiry test"
DEADLINE_BOUNTY_GITHUB="https://github.com/testuser/repo/issues/999"
DEADLINE_SECONDS=10
NOW_SECS=$(date +%s)
FUTURE_DEADLINE=$(( ($NOW_SECS + $DEADLINE_SECONDS) * 1000000000 ))

# Create bounty with short deadline
echo -e "\n${BLUE}========== [7] Deadline Expiry and Refund Test ==========${NC}"
echo -e "${GREEN}Creating bounty with deadline $DEADLINE_SECONDS seconds from now...${NC}"
DEADLINE_BOUNTY_RESULT=$(dfx canister call $BACKEND_ID create_bounty "(record {
  title = \"$DEADLINE_BOUNTY_TITLE\";
  description = \"$DEADLINE_BOUNTY_DESC\";
  github_issue_url = \"$DEADLINE_BOUNTY_GITHUB\";
  prize_amount = $PRIZE_AMOUNT;
  token_ledger = principal \"$LEDGER_ID\";
  from_subaccount = null;
  fee = null;
  deadline = opt $FUTURE_DEADLINE;
})")
echo "$DEADLINE_BOUNTY_RESULT"
DEADLINE_BOUNTY_ID=$(echo "$DEADLINE_BOUNTY_RESULT" | awk '/Ok = record/ {getline; match($0, /[0-9]+/); print substr($0, RSTART, RLENGTH); exit}')
DEADLINE_ESCROW_ACCOUNT=$(echo "$DEADLINE_BOUNTY_RESULT" | awk -F'"' '/Ok = record/ {getline; getline; print $2; exit}')
DEADLINE_ESCROW_SUBACCOUNT=$(echo "$DEADLINE_ESCROW_ACCOUNT" | grep -oE '[a-f0-9]{64}$')
DEADLINE_VECNAT8_SUBACCOUNT=$(hex_to_vecnat8 "$DEADLINE_ESCROW_SUBACCOUNT")

TRANSFER_ARG_FILE3=$(mktemp)
cat > "$TRANSFER_ARG_FILE3" <<EOF
(record {
  to = record {
    owner = principal "$BACKEND_ID";
    subaccount = $DEADLINE_VECNAT8_SUBACCOUNT;
  };
  amount = $TOTAL_AMOUNT;
  fee = null;
  memo = null;
  from_subaccount = null;
  created_at_time = null;
})
EOF
dfx canister call $LEDGER_ID icrc1_transfer --argument-file "$TRANSFER_ARG_FILE3"
rm "$TRANSFER_ARG_FILE3"

echo -e "${GREEN}Verifying escrow deposit for deadline bounty...${NC}"
dfx canister call $BACKEND_ID verify_escrow_deposit "($DEADLINE_BOUNTY_ID)"

echo -e "${YELLOW}Waiting $DEADLINE_SECONDS seconds for bounty to expire...${NC}"
sleep $DEADLINE_SECONDS

echo -e "${GREEN}Checking bounty status after deadline...${NC}"
dfx canister call $BACKEND_ID get_bounty "($DEADLINE_BOUNTY_ID)"

echo -e "${GREEN}Try to submit solution after deadline (should fail)...${NC}"
dfx canister call $BACKEND_ID submit_solution "(record { bounty_id = $DEADLINE_BOUNTY_ID; pr_url = \"https://github.com/testuser/repo/pull/expired\"; comment = \"Late submission\" })" || echo -e "${YELLOW}EXPECTED FAILURE: Submission after deadline prevented.${NC}"

echo -e "${GREEN}Try to select winner after deadline (should fail)...${NC}"
dfx canister call $BACKEND_ID select_winner "($DEADLINE_BOUNTY_ID, principal \"$PRINCIPAL\")" || echo -e "${YELLOW}EXPECTED FAILURE: Winner selection after deadline prevented.${NC}"

echo -e "${GREEN}Try to cancel bounty after deadline (should fail)...${NC}"
dfx canister call $BACKEND_ID cancel_bounty "($DEADLINE_BOUNTY_ID)" || echo -e "${YELLOW}EXPECTED FAILURE: Cancel after deadline prevented.${NC}"

echo -e "${GREEN}Try to refund expired bounty (should succeed)...${NC}"
dfx canister call $BACKEND_ID refund_expired_bounty "($DEADLINE_BOUNTY_ID)"

echo -e "${GREEN}Check bounty status after refund...${NC}"
dfx canister call $BACKEND_ID get_bounty "($DEADLINE_BOUNTY_ID)" 

echo -e "\n${GREEN}Deadline Expiry and Refund Test completed.${NC}\n"

echo -e "\n${BLUE}========== [8] Leaderboard/Stats Tests ==========${NC}"
echo -e "${GREEN}Top 5 creators:${NC}"
dfx canister call $BACKEND_ID get_top_creators "(5)"
echo -e "${GREEN}Top 5 winners:${NC}"
dfx canister call $BACKEND_ID get_top_winners "(5)"
echo -e "${GREEN}Top 5 participants:${NC}"
dfx canister call $BACKEND_ID get_top_participants "(5)"

echo -e "\n${GREEN}Leaderboard/Stats Tests completed.${NC}\n"


