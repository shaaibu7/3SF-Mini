use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use slot_finality::types::{Block, State, Config, ZERO_HASH};
use slot_finality::p2p::{P2PNetwork, Staker};

fn main() {
    println!("=== 3SF-Mini Simulation ===\n");
    
    // Create genesis block and state
    let num_validators = 4;
    let config = Config { num_validators };
    
    let genesis_state = State {
        config: config.clone(),
        latest_justified_hash: ZERO_HASH.to_string(),
        latest_justified_slot: 0,
        latest_finalized_hash: ZERO_HASH.to_string(),
        latest_finalized_slot: 0,
        historical_block_hashes: vec![Some(ZERO_HASH.to_string())],
        justified_slots: vec![true],
        justifications: HashMap::new(),
    };
    
    let genesis_block = Block {
        slot: 0,
        parent: None,
        votes: Vec::new(),
        state_root: Some(ZERO_HASH.to_string()),
    };
    
    // Initialize P2P network with simple latency function (1-2 time units)
    let network = Rc::new(RefCell::new(P2PNetwork::new(
        Box::new(|_time| 1 + (rand::random::<u64>() % 2))
    )));
    
    // Create 4 staker nodes
    let mut stakers = Vec::new();
    for validator_id in 0..num_validators {
        let staker = Staker::new(
            validator_id,
            network.clone(),
            genesis_block.clone(),
            genesis_state.clone(),
        );
        stakers.push(staker);
    }
    
    println!("Created {} validators", num_validators);
    println!("Genesis block: {}\n", ZERO_HASH);
    
    // Run simulation loop
    let simulation_time = 200;
    for _t in 0..simulation_time {
        let current_time = network.borrow().time;
        
        // Print progress every 12 time units (one slot)
        if current_time % 12 == 0 {
            let slot = current_time / 12;
            println!("--- Time: {}, Slot: {} ---", current_time, slot);
            
            // Print state of first validator as representative
            if let Some(staker_ref) = stakers.first() {
                let staker = staker_ref.borrow();
                let justified = staker.latest_justified_hash();
                let finalized = staker.latest_finalized_hash();
                
                println!("  Latest Justified: {}...", &justified[..16]);
                println!("  Latest Finalized: {}...", &finalized[..16]);
                println!("  Chain size: {} blocks", staker.chain.len());
            }
            println!();
        }
        
        // Tick each staker
        for staker_ref in &stakers {
            staker_ref.borrow_mut().tick();
        }
        
        // Advance network time
        network.borrow_mut().time_step();
    }
    
    println!("=== Simulation Complete ===");
    println!("Final time: {}", network.borrow().time);
    
    // Print final state
    if let Some(staker_ref) = stakers.first() {
        let staker = staker_ref.borrow();
        println!("\nFinal State (Validator 0):");
        println!("  Chain size: {} blocks", staker.chain.len());
        println!("  Latest Justified: {}...", &staker.latest_justified_hash()[..16]);
        println!("  Latest Finalized: {}...", &staker.latest_finalized_hash()[..16]);
        println!("  Known votes: {}", staker.known_votes.len());
    }
}
