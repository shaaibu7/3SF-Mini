use serde::{Deserialize, Serialize};
use std::collections::HashMap;


const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub num_validators: i64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub config: Config,
    pub latest_justified_hash: String,
    pub latest_justified_slot: i64,
    pub latest_finalized_hash: String,
    pub latest_finalized_slot: i64,
    pub historical_block_hashes: Vec<String>,
    pub justified_slots: Vec<bool>,
    pub justifications: HashMap<String, Vec<bool>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vote {
    pub validator_id: i64,
    pub slot: i64,
    pub head: String,
    pub head_slot: i64,
    pub target: String,
    pub target_slot: i64,
    pub source: String,
    pub source_slot: i64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub slot: i64,
    pub parent: Option<String>,
    pub votes: Vec<Vote>,
    pub state_root: Option<String>,
}

