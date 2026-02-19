use std::collections::HashMap;

use sha2::{Digest, Sha256};
use serde::Serialize;

use crate::types::{ZERO_HASH, Block, State, Vote};



/// Computes a deterministic SHA-256 hash of any serializable object.
/// 
/// This function serializes the object to JSON and computes its hash,
/// ensuring all nodes agree on object identifiers.
/// 
/// # Arguments
/// * `obj` - Any object that implements Serialize
/// 
/// # Returns
/// A 64-character hexadecimal string representing the SHA-256 hash
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
    // Check if n = x^2 + x for some x
    // This means x^2 + x - n = 0
    // Using quadratic formula: x = (-1 + sqrt(1 + 4n)) / 2
    // For n to be pronic, x must be a non-negative integer
    let discriminant = 1.0 + 4.0 * (n as f64);
    let sqrt_discriminant = discriminant.sqrt();
    
    // Check if discriminant is a perfect square
    if sqrt_discriminant.fract() != 0.0 {
        return false;
    }
    
    let x = (-1.0 + sqrt_discriminant) / 2.0;
    x >= 0.0 && x.fract() == 0.0 && (x as u64) * ((x as u64) + 1) == n
}


/// Determines if a slot is eligible for justification based on distance from finalized slot.
/// 
/// Slots are justifiable if the delta from the finalized slot is:
/// - ≤ 5 (recent slots)
/// - A perfect square (x²)
/// - An oblong number (x² + x)
/// 
/// This backoff mechanism ensures finality progresses even under high latency.
/// 
/// # Arguments
/// * `finalized_slot` - The latest finalized slot
/// * `candidate_slot` - The slot to check for justifiability
/// 
/// # Panics
/// Panics if candidate_slot < finalized_slot
pub fn is_justifiable_slot(finalized_slot: u64, candidate_slot: u64) -> bool {
    assert!(candidate_slot >= finalized_slot);
    
    let delta = candidate_slot - finalized_slot;
    
    delta <= 5 || is_perfect_square(delta.into()) || is_pronic(delta)
}

/// Processes a block and returns the new consensus state.
/// 
/// This function:
/// 1. Updates historical block hashes
/// 2. Processes votes from the block
/// 3. Tracks justification attempts
/// 4. Justifies blocks when 2/3 threshold is reached
/// 5. Finalizes blocks when appropriate
/// 
/// # Arguments
/// * `state` - The current consensus state
/// * `block` - The block to process
/// 
/// # Returns
/// A new State with updates from processing the block
pub fn process_block(state: State, block: &Block) -> State {
    let mut state_copy = state.clone();
    // Track historical blocks in the state
    state_copy.historical_block_hashes.push(block.parent.clone());
    state_copy.justified_slots.push(false);
    
    while state_copy.historical_block_hashes.len() < block.slot as usize {
        state_copy.justified_slots.push(false);
        state_copy.historical_block_hashes.push(None);
    }
    
    // process votes
    for vote in &block.votes {
        // Ignore votes whose source is not already justified,
        // or whose target is not in the history, or whose target is not a
        // valid justifiable slot
        
        if vote.source_slot as usize >= state_copy.justified_slots.len()
            || !state_copy.justified_slots[vote.source_slot as usize]
            || vote.target_slot as usize >= state_copy.historical_block_hashes.len()
            || state_copy.historical_block_hashes[vote.source_slot as usize] != Some(vote.source.clone())
            || state_copy.historical_block_hashes[vote.target_slot as usize] != Some(vote.target.clone())
            || vote.target_slot <= vote.source_slot
            || !is_justifiable_slot(state_copy.latest_finalized_slot, vote.target_slot) {
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
                            if is_justifiable_slot(state_copy.latest_finalized_slot, slot as u64) {
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

/// Gets the justified block with the highest slot number across all post-states.
/// 
/// # Arguments
/// * `post_states` - Map of block hashes to their post-states
/// 
/// # Returns
/// The hash of the justified block with the highest slot
/// 
/// # Panics
/// Panics if post_states is empty
pub fn get_latest_justified_hash(post_states: &HashMap<String, State>) -> String {
    post_states.values()
        .max_by_key(|s| s.latest_justified_slot)
        .expect("post state must not be empty")
        .latest_justified_hash
        .clone()
}

/// Implements LMD GHOST (Latest Message Driven Greedy Heaviest Observed SubTree) fork choice.
/// 
/// This algorithm:
/// 1. Starts from a root block (usually latest justified)
/// 2. Counts votes for each block (including descendant votes)
/// 3. Repeatedly selects the child with the most votes
/// 4. Tiebreaks by slot number, then hash lexicographically
/// 
/// # Arguments
/// * `blocks` - Map of all known blocks
/// * `root` - Starting block hash (or ZERO_HASH for genesis)
/// * `votes` - All votes to consider
/// * `min_score` - Minimum vote weight required for a block to be considered
/// 
/// # Returns
/// The hash of the chosen head block
pub fn get_fork_choice_head(
    blocks: &HashMap<String, Block>,
    root: &str,
    votes: &[Vote],
    min_score: usize
) -> String {
    // Start at genesis by default if root is ZERO_HASH
    let root = if root == ZERO_HASH {
        blocks.keys()
            .min_by_key(|hash| blocks[*hash].slot)
            .expect("blocks must not be empty")
            .clone()
    } else {
        root.to_string()
    };

    // Identify latest votes - keep only the most recent vote from each validator
    let mut latest_votes: HashMap<u64, &Vote> = HashMap::new();
    for vote in votes {
        latest_votes.entry(vote.validator_id)
            .and_modify(|existing| {
                if vote.slot > existing.slot {
                    *existing = vote;
                }
            })
            .or_insert(vote);
    }

    // For each block, count the number of votes for that block. A vote
    // for any descendant of a block also counts as a vote for that block
    let mut vote_weights: HashMap<String, usize> = HashMap::new();

    for vote in latest_votes.values() {
        if let Some(mut block_hash) = blocks.get(&vote.head).map(|_| vote.head.clone()) {
            // Walk up the chain from the vote's head to the root
            while let Some(block) = blocks.get(&block_hash) {
                if block.slot <= blocks[&root].slot {
                    break;
                }
                *vote_weights.entry(block_hash.clone()).or_insert(0) += 1;
                
                // Move to parent
                if let Some(parent) = &block.parent {
                    block_hash = parent.clone();
                } else {
                    break;
                }
            }
        }
    }

    // Identify the children of each block
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for (hash, block) in blocks {
        if let Some(parent) = &block.parent {
            if vote_weights.get(hash).copied().unwrap_or(0) >= min_score {
                children_map.entry(parent.clone())
                    .or_insert_with(Vec::new)
                    .push(hash.clone());
            }
        }
    }

    // Start at the root and repeatedly choose the child with the most latest votes,
    // tiebreaking by slot then hash
    let mut current = root;
    loop {
        let children = children_map.get(&current).cloned().unwrap_or_default();
        if children.is_empty() {
            return current;
        }
        
        // Select child with highest vote weight, tiebreak by slot then hash
        current = children.into_iter()
            .max_by_key(|hash| {
                let weight = vote_weights.get(hash).copied().unwrap_or(0);
                let slot = blocks[hash].slot;
                (weight, slot, hash.clone())
            })
            .expect("children must not be empty");
    }
}
