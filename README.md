# Avox - Decentralized GitHub Bounty Platform

Transform GitHub contributions into on-chain rewards. Avox enables open-source maintainers to create bounties for GitHub issues and reward contributors with cryptocurrency on the Internet Computer.

## 🚀 Project Overview
Avox is a decentralized bounty platform built on the Internet Computer (ICP). Maintainers can create bounties for GitHub issues and pay contributors in any ICRC-1 token (ICP, ckBTC, etc.) using a secure, on-chain escrow system.

## 🚀 Quick Start

```bash
# Install DFX
sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Clone and setup
git clone <your-repo>
cd avox

# Start local replica
dfx start --clean

# Deploy canisters
dfx deploy

# Run test script 
./scripts/test_flow.sh
```

## 📋 Project Status

**Current Phase**: Milestone 1 - Backend Development 
- ✅ Bounty creation and management
- ✅ ICRC-1 token escrow system  (multi-token, secure subaccounts)
- ✅ Submission and winner selection
- ✅ User profiles & leaderboards
- ✅ Pagination and filtering
- ✅ Health/status monitoring
- ✅ Deadline/expiry management
- ✅ Double-claim prevention
- ✅ Input validation
- ✅ Memory isolation & stable storage
- ⏳ Frontend (Milestone 2)

## 🏗️ Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Creator   │────▶│    Backend   │◀────│ Contributor │
└─────────────┘     │   Canister   │     └─────────────┘
                    │              │
                    │  - Bounties  │
                    │  - Escrow    │
                    │  - Transfers │
                    └──────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │ ICRC-1 Token │
                    │    Ledger    │
                    └──────────────┘
```

## 🔧 Core Features

- **Multi-Token Escrow:** Pay bounties in any ICRC-1 token (ICP, ckBTC, etc.) with secure, isolated subaccounts.
- **Full Bounty Lifecycle:** Create, fund, submit, select winner, claim, cancel, and refund bounties.
- **User Profiles & Leaderboards:** Track user stats and display top creators, winners, and participants.
- **Pagination & Filtering:** Efficiently query and discover bounties.
- **Health/Status Monitoring:** Check canister version, bounty count, and last update.
- **Deadline/Expiry Management:** Automatic handling of bounty deadlines and refunds.
- **Security:** Principal-based access control, input validation, double-claim prevention, and stable storage.

## 🛠️ Technical Stack

- **Backend**: Rust
- **Platform**: Internet Computer Protocol (ICP)
- **Token Standard**: ICRC-1
- **State Management**: StableBTreeMap (persistent across upgrades)
- **Escrow**: Subaccount-based isolation

## 🧪 Testing

### Automated Tests
```bash
# Run full test flow
./test_flow.sh

```

### Manual Testing
```bash


# Use dfx commands directly
dfx canister call avox_backend create_bounty '(record {
  title = "Fix memory leak";
  description = "Details here";
  github_issue_url = "https://github.com/example/repo/issues/1";
  prize_amount = 100000000;
  token_ledger = principal "<token-ledger-id>";
})'
```

## 📊 Bounty Lifecycle

| Condition                        | Action Allowed         |
|----------------------------------|-----------------------|
| Deadline passed, no winner       | Creator can refund    |
| Deadline passed, winner selected | Only winner can claim |
| Before deadline                  | Normal flow           |

1. **Create** → Maintainer creates bounty with GitHub issue URL and optional deadline
2. **Fund** → Maintainer deposits tokens to escrow account
3. **Submit** → Contributors submit PR links (before deadline)
4. **Select** → Maintainer selects winner (before deadline)
5. **Claim** → Winner withdraws reward (even after deadline, if selected)
6. **Refund** → If deadline passes and no winner, creator can refund

## 🚦 API Reference

### Update Calls
- `create_bounty(CreateBountyRequest) → Result<u64, String>`
- `verify_escrow_deposit(bounty_id: u64) → Result<bool, String>`
- `submit_solution(SubmitSolutionRequest) → Result<(), String>`
- `select_winner(bounty_id: u64, winner: Principal) → Result<(), String>`
- `claim_reward(bounty_id: u64) → Result<Nat, String>`
- `cancel_bounty(bounty_id: u64) → Result<(), String>`
- `refund_expired_bounty(bounty_id: u64) → Result<Nat, String>`

### Query Calls
- `get_bounty(bounty_id: u64) → Option<Bounty>`
- `get_all_bounties() → Vec<Bounty>`
- `get_active_bounties() → Vec<Bounty>`
- `get_user_created_bounties(user: Principal) → Vec<Bounty>`
- `get_user_submissions(user: Principal) → Vec<Bounty>`
- `get_escrow_account(bounty_id: u64) → Result<String, String>`
- `get_bounties_paginated(offset: u64, limit: u64) → Vec<Bounty>`
- `get_bounties_by_status(status: BountyStatus, offset: u64, limit: u64) → Vec<Bounty>`
- `get_status() → CanisterStatus`
- `get_top_creators(limit: u64) → Vec<UserProfile>`
- `get_top_winners(limit: u64) → Vec<UserProfile>`
- `get_top_participants(limit: u64) → Vec<UserProfile>`

## 🎯 Roadmap

### Milestone 1 (Current) - Backend
- [x] Core bounty management
- [x] ICRC-1 escrow system
- [x] Submission handling
- [x] Winner selection
- [x] Fund transfers
- [x] Testing suite

### Milestone 2 - Frontend
- [ ] Next.js web interface
- [ ] Wallet integration
- [ ] Bounty discovery
- [ ] User dashboard
- [ ] Responsive design

Built with ❤️ on the Internet Computer