use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk_macros::*;

use ic_cdk::{update, query};
use crate::types::*;
use crate::state::*;


mod types;
mod state;
mod escrow;

const CANISTER_VERSION: &str = "1.0.0";
use std::cell::Cell;
thread_local! {
    static LAST_UPDATED: Cell<u64> = Cell::new(0);
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct CanisterStatus {
    pub version: String,
    pub bounty_count: u64,
    pub last_updated: u64,
}

fn update_last_updated() {
    let now = ic_cdk::api::time();
    LAST_UPDATED.with(|cell| cell.set(now));
}

#[ic_cdk::init]
fn init() {
    ic_cdk::println!("Avox Backend Canister Initialized");
}

#[ic_cdk::query]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

/// Update or create the caller's user profile.
#[update]
pub fn update_user_profile(request: UpdateUserProfileRequest) -> Result<(), String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    USER_PROFILES.with(|profiles| {
        let mut profiles = profiles.borrow_mut();
        let mut profile = profiles.get(&caller).map(|x| x.clone()).unwrap_or_else(|| UserProfile::default());
        if let Some(name) = request.name { profile.name = Some(name); }
        if let Some(github) = request.github { profile.github = Some(github); }
        if let Some(twitter) = request.twitter { profile.twitter = Some(twitter); }
        if let Some(pfp_url) = request.pfp_url { profile.pfp_url = Some(pfp_url); }
        profiles.insert(caller, profile);
    });
    Ok(())
}

/// Get a user's profile and stats.
#[query]
pub fn get_user_profile(user: Principal) -> Option<UserProfile> {
    USER_PROFILES.with(|profiles| profiles.borrow().get(&user).map(|x| x.clone()))
}

fn is_bounty_expired(bounty: &Bounty) -> bool {
    match bounty.deadline {
        Some(deadline) => ic_cdk::api::time() > deadline,
        None => false,
    }
}

#[update]
pub async fn create_bounty(request: CreateBountyRequest) -> Result<(u64, String), String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    if request.title.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if request.github_issue_url.is_empty() {
        return Err("GitHub issue URL cannot be empty".to_string());
    }
    if request.prize_amount == 0u64 {
        return Err("Prize amount must be greater than 0".to_string());
    }
    let now = ic_cdk::api::time();
    let bounty_id = BOUNTY_COUNTER.with(|counter| {
        let mut counter_ref = counter.borrow_mut();
        let current = *counter_ref.get();
        let new_id = current + 1;
        counter_ref.set(new_id).expect("Failed to update counter");
        new_id
    });
    let subaccount = escrow::generate_subaccount(bounty_id);
    let escrow_account = escrow::get_escrow_account(subaccount);
    let bounty = Bounty {
        id: bounty_id,
        creator: caller,
        title: request.title.clone(),
        description: request.description.clone(),
        github_issue_url: request.github_issue_url.clone(),
        prize_amount: request.prize_amount.clone(),
        token_ledger: request.token_ledger,
        status: BountyStatus::PendingFunding,
        submissions: vec![],
        winner: None,
        created_at: now,
        escrow_subaccount: subaccount,
        deadline: request.deadline,
    };
    BOUNTIES.with(|bounties| {
        bounties.borrow_mut().insert(bounty_id, bounty);
    });
    Ok((bounty_id, escrow_account.to_string()))
}

#[query]
pub fn get_escrow_account(bounty_id: u64) -> Result<String, String> {
    BOUNTIES.with(|bounties| {
        if let Some(bounty) = bounties.borrow().get(&bounty_id) {
            let account = escrow::get_escrow_account(bounty.escrow_subaccount);
            Ok(account.to_string())
        } else {
            Err("Bounty not found".to_string())
        }
    })
}

#[update]
pub async fn verify_escrow_deposit(bounty_id: u64) -> Result<bool, String> {
    update_last_updated();
    let mut funded = false;
    let mut token_ledger = None;
    let mut subaccount = None;
    let mut prize_amount = None;
    let mut deadline = None;
    let mut status = None;
    BOUNTIES.with(|bounties| {
        if let Some(bounty) = bounties.borrow().get(&bounty_id) {
            token_ledger = Some(bounty.token_ledger);
            subaccount = Some(bounty.escrow_subaccount);
            prize_amount = Some(bounty.prize_amount.clone());
            deadline = bounty.deadline;
            status = Some(bounty.status.clone());
        }
    });
    let (token_ledger, subaccount, prize_amount, deadline, status) = match (token_ledger, subaccount, prize_amount, deadline, status) {
        (Some(t), Some(s), Some(p), d, s2) => (t, s, p, d, s2),
        _ => return Err("Bounty not found".to_string()),
    };
    if let Some(deadline) = deadline {
        if ic_cdk::api::time() > deadline && status == Some(BountyStatus::Active) {
            // Mark as expired
            BOUNTIES.with(|bounties| {
                let mut bounties_ref = bounties.borrow_mut();
                if let Some(mut bounty) = bounties_ref.get(&bounty_id) {
                    bounty.status = BountyStatus::Expired;
                    bounties_ref.insert(bounty_id, bounty);
                }
            });
            return Ok(false);
        }
    }
    let balance = escrow::check_balance(token_ledger, subaccount).await?;
    if balance >= prize_amount {
        // Mark bounty as Active if not already
        BOUNTIES.with(|bounties| {
            let mut bounties_ref = bounties.borrow_mut();
            if let Some(bounty) = bounties_ref.get(&bounty_id) {
                if bounty.status != BountyStatus::Active {
                    let mut updated_bounty = bounty.clone();
                    updated_bounty.status = BountyStatus::Active;
                    bounties_ref.insert(bounty_id, updated_bounty);
                }
            }
        });
        funded = true;
    }
    Ok(funded)
}

macro_rules! check_expired_and_update {
    ($bounty:expr, $bounty_id:expr) => {
        if is_bounty_expired(&$bounty) && $bounty.status == BountyStatus::Active {
            BOUNTIES.with(|bounties| {
                let mut bounties_ref = bounties.borrow_mut();
                let mut updated = $bounty.clone();
                updated.status = BountyStatus::Expired;
                bounties_ref.insert($bounty_id, updated);
            });
            return Err("Bounty has expired".to_string());
        }
    };
}

#[update]
pub fn submit_solution(request: SubmitSolutionRequest) -> Result<(), String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    if request.pr_url.is_empty() {
        return Err("PR URL cannot be empty".to_string());
    }
    BOUNTIES.with(|bounties| {
        let mut bounties_ref = bounties.borrow_mut();
        if let Some(mut bounty) = bounties_ref.get(&request.bounty_id) {
            check_expired_and_update!(bounty, request.bounty_id);
            if bounty.status != BountyStatus::Active {
                return Err("Bounty is not active".to_string());
            }
            if bounty.submissions.iter().any(|s| s.submitter == caller) {
                return Err("You have already submitted a solution".to_string());
            }
            let submission = Submission {
                submitter: caller,
                pr_url: request.pr_url,
                comment: request.comment,
                submitted_at: ic_cdk::api::time(),
            };
            bounty.submissions.push(submission);
            bounties_ref.insert(request.bounty_id, bounty);
            // Track user submission
            USER_SUBMISSIONS.with(|user_subs| {
                let mut us = user_subs.borrow_mut();
                let mut sub_list = us.get(&caller).map(|x| x.clone()).unwrap_or(BountyIdList(vec![]));
                sub_list.0.push(request.bounty_id);
                us.insert(caller, sub_list);
            });
            // Increment bounties_participated in user profile (only once per bounty)
            let is_first_submission = USER_SUBMISSIONS.with(|user_subs| {
                let user_subs = user_subs.borrow();
                !user_subs.get(&caller).map(|x| !x.0.is_empty()).unwrap_or(true)
            });
            if is_first_submission {
                USER_PROFILES.with(|profiles| {
                    let mut profiles = profiles.borrow_mut();
                    let mut profile = profiles.get(&caller).map(|x| x.clone()).unwrap_or_else(|| UserProfile::default());
                    profile.bounties_participated += 1;
                    profiles.insert(caller, profile);
                });
            }
            Ok(())
        } else {
            Err("Bounty not found".to_string())
        }
    })
}

#[update]
pub async fn select_winner(bounty_id: u64, winner: Principal) -> Result<(), String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    let bounty = BOUNTIES.with(|bounties| {
        bounties.borrow().get(&bounty_id)
    }).ok_or("Bounty not found")?;
    check_expired_and_update!(bounty, bounty_id);
    if bounty.creator != caller {
        return Err("Only bounty creator can select winner".to_string());
    }
    if bounty.status != BountyStatus::Active {
        return Err("Bounty is not active".to_string());
    }
    if !bounty.submissions.iter().any(|s| s.submitter == winner) {
        return Err("Selected winner has not submitted a solution".to_string());
    }
    // Update bounty
    BOUNTIES.with(|bounties| {
        let mut bounties_ref = bounties.borrow_mut();
        if let Some(mut bounty) = bounties_ref.get(&bounty_id) {
            bounty.winner = Some(winner);
            bounty.status = BountyStatus::Completed;
            bounties_ref.insert(bounty_id, bounty);
        }
    });
    // Increment bounties_won in winner's profile
    USER_PROFILES.with(|profiles| {
        let mut profiles = profiles.borrow_mut();
        let mut profile = profiles.get(&winner).map(|x| x.clone()).unwrap_or_else(|| UserProfile::default());
        profile.bounties_won += 1;
        profiles.insert(winner, profile);
    });
    Ok(())
}

#[update]
pub async fn claim_reward(bounty_id: u64) -> Result<Nat, String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    let claim_key = ClaimKey {
        bounty_id,
        principal: caller,
    };
    if CLAIMS.with(|claims| claims.borrow().get(&claim_key)).is_some() {
        return Err("Reward already claimed".to_string());
    }
    let bounty = BOUNTIES.with(|bounties| {
        bounties.borrow().get(&bounty_id)
    }).ok_or("Bounty not found")?;
    check_expired_and_update!(bounty, bounty_id);
    match bounty.winner {
        Some(winner) if winner == caller => {},
        _ => return Err("You are not the winner of this bounty".to_string()),
    }
    if bounty.status != BountyStatus::Completed {
        return Err("Bounty is not completed".to_string());
    }
    let transfer_result = escrow::transfer_from_escrow(
        bounty.token_ledger,
        bounty.escrow_subaccount,
        caller,
        bounty.prize_amount.clone()
    ).await?;
    CLAIMS.with(|claims| {
        claims.borrow_mut().insert(claim_key, true);
    });
    Ok(transfer_result)
}

#[update]
pub async fn refund_expired_bounty(bounty_id: u64) -> Result<Nat, String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    let bounty = BOUNTIES.with(|bounties| {
        bounties.borrow().get(&bounty_id)
    }).ok_or("Bounty not found")?;
    if bounty.creator != caller {
        return Err("Only the creator can refund the bounty".to_string());
    }
    if bounty.status != BountyStatus::Expired {
        return Err("Bounty is not expired; cannot refund unless status is Expired".to_string());
    }
    if bounty.winner.is_some() {
        return Err("Winner already selected, cannot refund".to_string());
    }
    let refund_result = escrow::transfer_from_escrow(
        bounty.token_ledger,
        bounty.escrow_subaccount,
        caller,
        bounty.prize_amount.clone()
    ).await?;
    Ok(refund_result)
}

#[update]
pub async fn cancel_bounty(bounty_id: u64) -> Result<(), String> {
    update_last_updated();
    let caller = ic_cdk::caller();
    let bounty = BOUNTIES.with(|bounties| {
        bounties.borrow().get(&bounty_id)
    }).ok_or("Bounty not found")?;
    check_expired_and_update!(bounty, bounty_id);
    if bounty.creator != caller {
        return Err("Only bounty creator can cancel".to_string());
    }
    if bounty.status != BountyStatus::Active {
        return Err("Can only cancel active bounties".to_string());
    }
    if !bounty.submissions.is_empty() {
        return Err("Cannot cancel bounty with submissions".to_string());
    }
    let _refund_result = escrow::transfer_from_escrow(
        bounty.token_ledger,
        bounty.escrow_subaccount,
        caller,
        bounty.prize_amount.clone()
    ).await?;
    BOUNTIES.with(|bounties| {
        let mut bounties_ref = bounties.borrow_mut();
        if let Some(mut bounty) = bounties_ref.get(&bounty_id) {
            bounty.status = BountyStatus::Cancelled;
            bounties_ref.insert(bounty_id, bounty);
        }
    });
    Ok(())
}

#[query]
pub fn get_bounty(bounty_id: u64) -> Option<Bounty> {
    BOUNTIES.with(|bounties| bounties.borrow().get(&bounty_id).map(|x| x.clone()))
}

#[query]
pub fn get_all_bounties() -> Vec<Bounty> {
    BOUNTIES.with(|bounties| {
        bounties.borrow().iter().map(|(_, b)| b.clone()).collect()
    })
}

#[query]
pub fn get_active_bounties() -> Vec<Bounty> {
    BOUNTIES.with(|bounties| {
        bounties.borrow()
            .iter()
            .filter(|(_, b)| b.status == BountyStatus::Active)
            .map(|(_, b)| b.clone())
            .collect()
    })
}

#[query]
pub fn get_user_created_bounties(user: Principal) -> Vec<Bounty> {
    USER_CREATED_BOUNTIES.with(|user_bounties| {
        if let Some(bounty_list) = user_bounties.borrow().get(&user) {
            BOUNTIES.with(|bounties| {
                let bounties_ref = bounties.borrow();
                bounty_list.0.iter()
                    .filter_map(|id| bounties_ref.get(id).map(|x| x.clone()))
                    .collect()
            })
        } else {
            vec![]
        }
    })
}

#[query]
pub fn get_user_submissions(user: Principal) -> Vec<Bounty> {
    USER_SUBMISSIONS.with(|user_subs| {
        if let Some(bounty_list) = user_subs.borrow().get(&user) {
            BOUNTIES.with(|bounties| {
                let bounties_ref = bounties.borrow();
                bounty_list.0.iter()
                    .filter_map(|id| bounties_ref.get(id).map(|x| x.clone()))
                    .collect()
            })
        } else {
            vec![]
        }
    })
}

#[query]
pub fn get_bounties_paginated(offset: u64, limit: u64) -> Vec<Bounty> {
    BOUNTIES.with(|bounties| {
        bounties.borrow()
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, b)| b.clone())
            .collect()
    })
}

#[query]
pub fn get_bounties_by_status(status: BountyStatus, offset: u64, limit: u64) -> Vec<Bounty> {
    BOUNTIES.with(|bounties| {
        bounties.borrow()
            .iter()
            .filter(|(_, b)| b.status == status)
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, b)| b.clone())
            .collect()
    })
}

#[query]
pub fn get_status() -> CanisterStatus {
    let bounty_count = BOUNTIES.with(|b| b.borrow().len() as u64);
    let last_updated = LAST_UPDATED.with(|cell| cell.get());
    CanisterStatus {
        version: CANISTER_VERSION.to_string(),
        bounty_count,
        last_updated,
    }
}

#[query]
pub fn get_top_creators(limit: u64) -> Vec<UserProfile> {
    USER_PROFILES.with(|profiles| {
        let mut all: Vec<_> = profiles.borrow().values().collect();
        all.sort_by(|a, b| b.bounties_posted.cmp(&a.bounties_posted));
        all.into_iter().take(limit as usize).collect()
    })
}

#[query]
pub fn get_top_winners(limit: u64) -> Vec<UserProfile> {
    USER_PROFILES.with(|profiles| {
        let mut all: Vec<_> = profiles.borrow().values().collect();
        all.sort_by(|a, b| b.bounties_won.cmp(&a.bounties_won));
        all.into_iter().take(limit as usize).collect()
    })
}

#[query]
pub fn get_top_participants(limit: u64) -> Vec<UserProfile> {
    USER_PROFILES.with(|profiles| {
        let mut all: Vec<_> = profiles.borrow().values().collect();
        all.sort_by(|a, b| b.bounties_participated.cmp(&a.bounties_participated));
        all.into_iter().take(limit as usize).collect()
    })
}

ic_cdk::export_candid!();
