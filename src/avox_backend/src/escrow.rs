use candid::{Nat, Principal};
use ic_cdk::api::call::call;
use sha2::{Sha256, Digest};

// ICRC-1 Types
#[derive(candid::CandidType, candid::Deserialize)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<[u8; 32]>,
}

impl Account {
    pub fn to_string(&self) -> String {
        if let Some(subaccount) = self.subaccount {
            // Only show non-zero bytes of subaccount
            let non_zero_bytes: Vec<u8> = subaccount.iter()
                .rev()
                .skip_while(|&&b| b == 0)
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            
            if non_zero_bytes.is_empty() {
                self.owner.to_string()
            } else {
                format!("{}-{}", self.owner, hex::encode(non_zero_bytes))
            }
        } else {
            self.owner.to_string()
        }
    }
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct TransferArg {
    pub from_subaccount: Option<[u8; 32]>,
    pub to: Account,
    pub fee: Option<Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
    pub amount: Nat,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
pub enum TransferError {
    BadFee { expected_fee: Nat },
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub type TransferResult = Result<Nat, TransferError>;

// Generate a unique subaccount for each bounty
pub fn generate_subaccount(bounty_id: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"avox-bounty-");
    hasher.update(bounty_id.to_be_bytes());
    let result = hasher.finalize();
    let mut subaccount = [0u8; 32];
    subaccount.copy_from_slice(&result);
    subaccount
}

// Get the escrow account for a specific bounty
pub fn get_escrow_account(subaccount: [u8; 32]) -> Account {
    Account {
        owner: ic_cdk::id(),
        subaccount: Some(subaccount),
    }
}

// Check balance of a subaccount
pub async fn check_balance(
    token_ledger: Principal,
    subaccount: [u8; 32],
) -> Result<Nat, String> {
    let account = Account {
        owner: ic_cdk::id(),
        subaccount: Some(subaccount),
    };
    
    let result: Result<(Nat,), _> = call(
        token_ledger,
        "icrc1_balance_of",
        (account,)
    ).await;
    
    match result {
        Ok((balance,)) => Ok(balance),
        Err(e) => Err(format!("Failed to check balance: {:?}", e)),
    }
}

// Transfer funds from escrow to winner
pub async fn transfer_from_escrow(
    token_ledger: Principal,
    from_subaccount: [u8; 32],
    to: Principal,
    amount: Nat,
) -> Result<Nat, String> {
    let transfer_args = TransferArg {
        from_subaccount: Some(from_subaccount),
        to: Account {
            owner: to,
            subaccount: None,
        },
        fee: None,
        memo: None,
        created_at_time: None,
        amount,
    };
    
    let result: Result<(TransferResult,), _> = call(
        token_ledger,
        "icrc1_transfer",
        (transfer_args,)
    ).await;
    
    match result {
        Ok((Ok(block_index),)) => Ok(block_index),
        Ok((Err(e),)) => Err(format!("Transfer failed: {:?}", e)),
        Err(e) => Err(format!("Inter-canister call failed: {:?}", e)),
    }
}

// Helper function to create transfer instructions for users
pub fn get_deposit_instructions(bounty_id: u64) -> String {
    let subaccount = generate_subaccount(bounty_id);
    let account = get_escrow_account(subaccount);
    
    format!(
        "To fund this bounty, transfer tokens to:\n\
         Account: {}\n\
         Owner: {}\n\
         Subaccount: {}",
        account.to_string(),
        ic_cdk::id(),
        hex::encode(subaccount)
    )
} 