use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableCell};
use std::cell::RefCell;
use crate::types::{Bounty, BountyIdList, ClaimKey, UserProfile};
use candid::Principal;

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // Memory ID 0: Bounties storage
    pub static BOUNTIES: RefCell<StableBTreeMap<u64, Bounty, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );

    // Memory ID 1: Bounty counter
    pub static BOUNTY_COUNTER: RefCell<StableCell<u64, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
            0
        ).expect("Failed to initialize bounty counter")
    );

    // Memory ID 2: User submissions mapping
    pub static USER_SUBMISSIONS: RefCell<StableBTreeMap<Principal, BountyIdList, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
        )
    );

    // Memory ID 3: User created bounties mapping
    pub static USER_CREATED_BOUNTIES: RefCell<StableBTreeMap<Principal, BountyIdList, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
        )
    );

    // Memory ID 4: Claims tracking
    pub static CLAIMS: RefCell<StableBTreeMap<ClaimKey, bool, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))),
        )
    );

    // Memory ID 5: User profiles
    pub static USER_PROFILES: RefCell<StableBTreeMap<Principal, UserProfile, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))),
        )
    );
} 