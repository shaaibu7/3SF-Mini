use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub const SLOT_DURATION: u64 = 12;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub num_validators: u64
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub config: Config,
    pub latest_justified_hash: String,
    pub latest_justified_slot: u64,
    pub latest_finalized_hash: String,
    pub latest_finalized_slot: u64,
    pub historical_block_hashes: Vec<Option<String>>,
    pub justified_slots: Vec<bool>,
    pub justifications: HashMap<String, Vec<bool>>
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Vote {
    pub validator_id: u64,
    pub slot: u64,
    pub head: String,
    pub head_slot: u64,
    pub target: String,
    pub target_slot: u64,
    pub source: String,
    pub source_slot: u64
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub slot: u64,
    pub parent: Option<String>,
    pub votes: Vec<Vote>,
    pub state_root: Option<String>,
}

