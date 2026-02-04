use sha2::{Digest, Sha256};
use serde::Serialize;

use crate::types::{ZERO_HASH, Block, State};



// Stub for computing block hash, state root...
// (in real life replace with SSZ hashin
pub fn compute_hash<T: Serialize>(obj: &T) -> String {
    // Serialize to JSON with sorted keys
    let serialized = serde_json::to_string(obj).unwrap();

    // Hash
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());

    // Return hex digest
    format!("{:x}", hasher.finalize())
}

fn is_perfect_square(n: u64) -> bool {
    let x = (n as f64).sqrt() as u64;
    x * x == n
}

fn is_pronic(n: u64) -> bool {
    let x = (n as f64).sqrt() as u64;
    x * (x + 1) == n || (x + 1) * (x + 2) == n
}


// We allow justification of slots either <= 5 or a perfect square or oblong after
// the latest finalized slot. This gives us a backoff technique and ensures
// finality keeps progressing even under high latency
pub fn is_justifiable_slot(finalized_slot: u64, candidate_slot: u64) -> bool {
    assert!(candidate_slot >= finalized_slot);
    
    let delta = candidate_slot - finalized_slot;
    
    delta <= 5 || is_perfect_square(delta.into()) || is_pronic(delta)
}

// Given a state, output the new state after processing that block
pub fn process_block(state: State, block: Block) -> State {
    let mut state_copy = state.clone();
    // Track historical blocks in the state
    state_copy.historical_block_hashes.push(block.parent);
    state_copy.justified_slots.push(false);
    
    while state_copy.historical_block_hashes.len() < block.slot as usize {
        state_copy.justified_slots.push(false);
        state_copy.historical_block_hashes.push(String::from(ZERO_HASH));
    }
    
    // process votes
    for vote in &block.votes {
        // Ignore votes whose source is not already justified,
        // or whose target is not in the history, or whose target is not a
        // valid justifiable slot
        
        if !state_copy.justified_slots[vote.source_slot as usize]
            || vote.source != state_copy.historical_block_hashes[vote.source_slot as usize]
            || vote.target != state_copy.historical_block_hashes[vote.target_slot as usize]
            || vote.target_slot <= vote.source_slot
            || !is_justifiable_slot(state.latest_finalized_slot, vote.target_slot) {
                continue;
            
        }
            
            // Track attempts to justify new hashes
            state_copy.justifications.entry(vote.target.clone())
                .or_insert_with(|| vec![false; state.config.num_validators as usize]);
            
            
            // Get mutable reference to vote tracking vector
            let votes_for_target = state_copy.justifications
                .get_mut(&vote.target)
                .expect("justification entry must exist");
            
            // Mark this validator as having voted
            if !votes_for_target[vote.validator_id as usize] {
                votes_for_target[vote.validator_id as usize] = true;
            }
                
            // Count how many validators voted for this target
            let count = votes_for_target.iter().filter(|&&v| v).count();
            
            // If 2/3 of validators voted for this target, justify it
                    if count == (2 * state_copy.config.num_validators as usize) / 3 {
                        state_copy.latest_justified_hash = vote.target.clone();
                        state_copy.latest_justified_slot = vote.target_slot as u64;
                        state_copy.justified_slots[vote.target_slot as usize] = true;
            
                        // Remove the entry from justifications map
                        state_copy.justifications.remove(&vote.target);
            
                        // Finalization: check for any intermediate justifiable slots
                        let mut has_intermediate_justifiable = false;
                        for slot in (vote.source_slot + 1)..vote.target_slot {
                            if is_justifiable_slot(state.latest_finalized_slot, slot as u64) {
                                has_intermediate_justifiable = true;
                                break;
                            }
                        }
            
                        if !has_intermediate_justifiable {
                            state_copy.latest_finalized_hash = vote.source.clone();
                            state_copy.latest_finalized_slot = vote.source_slot as u64;
                        }
                    }
        }
    
    
    state_copy
    
    
}