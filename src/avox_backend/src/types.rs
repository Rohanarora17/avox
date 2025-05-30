use candid::{CandidType, Deserialize, Nat, Principal};
use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use std::borrow::Cow;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Bounty {
    pub id: u64,
    pub creator: Principal,
    pub title: String,
    pub description: String,
    pub github_issue_url: String,
    pub prize_amount: Nat,
    pub token_ledger: Principal,
    pub status: BountyStatus,
    pub submissions: Vec<Submission>,
    pub winner: Option<Principal>,
    pub created_at: u64,
    pub escrow_subaccount: [u8; 32],
    pub deadline: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq)]
pub enum BountyStatus {
    PendingFunding,
    Active,
    Completed,
    Cancelled,
    Expired,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Submission {
    pub submitter: Principal,
    pub pr_url: String,
    pub comment: String,
    pub submitted_at: u64,
}

#[derive(CandidType, Deserialize)]
pub struct CreateBountyRequest {
    pub title: String,
    pub description: String,
    pub github_issue_url: String,
    pub prize_amount: Nat,
    pub token_ledger: Principal, // ICRC-1 token ledger canister ID
    pub from_subaccount: Option<[u8; 32]>, // NEW: for transfer
    pub fee: Option<Nat>,                  // NEW: for transfer
    pub deadline: Option<u64>, // NEW: nanoseconds since epoch
}

#[derive(CandidType, Deserialize)]
pub struct SubmitSolutionRequest {
    pub bounty_id: u64,
    pub pr_url: String,
    pub comment: String,
}

// User profile types
#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub principal: Principal,
    pub name: Option<String>,
    pub github: Option<String>,
    pub twitter: Option<String>,
    pub pfp_url: Option<String>,
    pub bounties_posted: u64,
    pub bounties_participated: u64,
    pub bounties_won: u64,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            principal: Principal::anonymous(),
            name: None,
            github: None,
            twitter: None,
            pfp_url: None,
            bounties_posted: 0,
            bounties_participated: 0,
            bounties_won: 0,
        }
    }
}

impl Storable for UserProfile {
    fn to_bytes(&self) -> Cow<[u8]> {
        match candid::encode_one(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(_) => Cow::Owned(vec![]),
        }
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| UserProfile::default())
    }
    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize)]
pub struct UpdateUserProfileRequest {
    pub name: Option<String>,
    pub github: Option<String>,
    pub twitter: Option<String>,
    pub pfp_url: Option<String>,
}

// Implement Storable for types to use in stable structures
impl Storable for Bounty {
    fn to_bytes(&self) -> Cow<[u8]> {
        match candid::encode_one(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(_) => Cow::Owned(vec![]),
        }
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| Bounty {
            id: 0,
            creator: Principal::anonymous(),
            title: String::new(),
            description: String::new(),
            github_issue_url: String::new(),
            prize_amount: Nat::from(0u64),
            token_ledger: Principal::anonymous(),
            status: BountyStatus::PendingFunding,
            submissions: vec![],
            winner: None,
            created_at: 0,
            escrow_subaccount: [0u8; 32],
            deadline: None,
        })
    }
    const BOUND: Bound = Bound::Unbounded;
}

// Simple wrapper for Vec<u64> to implement Storable
#[derive(CandidType, Deserialize, Clone)]
pub struct BountyIdList(pub Vec<u64>);

impl Storable for BountyIdList {
    fn to_bytes(&self) -> Cow<[u8]> {
        match candid::encode_one(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(_) => Cow::Owned(vec![]),
        }
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| BountyIdList(vec![]))
    }
    const BOUND: Bound = Bound::Unbounded;
}

// Wrapper for claim tracking
#[derive(CandidType, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClaimKey {
    pub bounty_id: u64,
    pub principal: Principal,
}

impl Storable for ClaimKey {
    fn to_bytes(&self) -> Cow<[u8]> {
        match candid::encode_one(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(_) => Cow::Owned(vec![]),
        }
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap_or_else(|_| ClaimKey { bounty_id: 0, principal: Principal::anonymous() })
    }
    const BOUND: Bound = Bound::Unbounded;
} 