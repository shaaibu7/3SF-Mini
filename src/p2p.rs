use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::types::{Block, Vote, State, SLOT_DURATION};
use crate::consensus::{compute_hash, get_latest_justified_hash};

/// Message type that can be sent over the P2P network
#[derive(Debug, Clone)]
pub enum Item {
    Block(Block),
    Vote(Vote),
}

/// P2P network simulator that manages message delivery between staker nodes
pub struct P2PNetwork {
    pub time: u64,
    stakers: HashMap<u64, Rc<RefCell<Staker>>>,
    queues: HashMap<u64, Vec<(u64, Item)>>,
    latency_func: Box<dyn Fn(u64) -> u64>,
}

impl P2PNetwork {
    /// Creates a new P2P network with a configurable latency function
    /// 
    /// # Arguments
    /// * `latency_func` - Function that takes current time and returns message delay
    pub fn new(latency_func: Box<dyn Fn(u64) -> u64>) -> Self {
        P2PNetwork {
            time: 0,
            stakers: HashMap::new(),
            queues: HashMap::new(),
            latency_func,
        }
    }

    /// Registers a staker node with the network
    pub fn register_staker(&mut self, staker: Rc<RefCell<Staker>>) {
        let validator_id = staker.borrow().validator_id;
        self.stakers.insert(validator_id, staker);
    }

    /// Submits a message to be delivered to all other stakers
    /// 
    /// # Arguments
    /// * `item` - The message (Block or Vote) to send
    /// * `sender_id` - The validator ID of the sender (won't receive their own message)
    pub fn submit(&mut self, item: Item, sender_id: u64) {
        for (&recipient_id, _) in &self.stakers {
            if recipient_id == sender_id {
                continue;
            }
            let deliver_at = self.time + (self.latency_func)(self.time);
            self.queues.entry(recipient_id)
                .or_insert_with(Vec::new)
                .push((deliver_at, item.clone()));
        }
    }

    /// Advances network time by one unit and delivers all pending messages
    pub fn time_step(&mut self) {
        self.time += 1;
        
        // Deliver messages scheduled for this time or earlier
        for (&validator_id, queue) in &mut self.queues {
            let (deliver_now, keep_later): (Vec<_>, Vec<_>) = 
                queue.drain(..).partition(|(t, _)| *t <= self.time);
            
            *queue = keep_later;
            
            if let Some(staker) = self.stakers.get(&validator_id) {
                for (_, item) in deliver_now {
                    staker.borrow_mut().receive(item);
                }
            }
        }
    }

    pub fn get_stakers(&self) -> &HashMap<u64, Rc<RefCell<Staker>>> {
        &self.stakers
    }
}

/// A validator node that participates in consensus by proposing blocks and voting
pub struct Staker {
    pub validator_id: u64,
    network: Rc<RefCell<P2PNetwork>>,
    pub chain: HashMap<String, Block>,
    post_states: HashMap<String, State>,
    pub known_votes: Vec<Vote>,
    new_votes: Vec<Vote>,
    dependencies: HashMap<String, Vec<Item>>,
    genesis_hash: String,
    num_validators: usize,
    safe_target: Option<String>,
    head: String,
}

impl Staker {
    /// Creates a new staker node and registers it with the network
    /// 
    /// # Arguments
    /// * `validator_id` - Unique identifier for this validator
    /// * `network` - Reference to the P2P network
    /// * `genesis_block` - The genesis block
    /// * `genesis_state` - The genesis consensus state
    /// 
    /// # Returns
    /// A reference-counted staker node
    pub fn new(
        validator_id: u64,
        network: Rc<RefCell<P2PNetwork>>,
        genesis_block: Block,
        genesis_state: State,
    ) -> Rc<RefCell<Self>> {
        let genesis_hash = compute_hash(&genesis_block);
        let num_validators = genesis_state.config.num_validators as usize;
        
        let mut chain = HashMap::new();
        chain.insert(genesis_hash.clone(), genesis_block);
        
        let mut post_states = HashMap::new();
        post_states.insert(genesis_hash.clone(), genesis_state);
        
        let staker = Rc::new(RefCell::new(Staker {
            validator_id,
            network: network.clone(),
            chain,
            post_states,
            known_votes: Vec::new(),
            new_votes: Vec::new(),
            dependencies: HashMap::new(),
            genesis_hash: genesis_hash.clone(),
            num_validators,
            safe_target: None,
            head: genesis_hash,
        }));
        
        // Register with network
        network.borrow_mut().register_staker(staker.clone());
        
        staker
    }

    /// Returns the hash of the latest justified block across all known chains
    pub fn latest_justified_hash(&self) -> String {
        get_latest_justified_hash(&self.post_states)
    }

    /// Returns the hash of the latest finalized block
    pub fn latest_finalized_hash(&self) -> String {
        self.post_states[&self.head].latest_finalized_hash.clone()
    }

    /// Returns the current slot number based on network time
    pub fn get_current_slot(&self) -> u64 {
        self.network.borrow().time / SLOT_DURATION + 2
    }

    fn recompute_head(&mut self) {
        let justified_hash = self.latest_justified_hash();
        self.head = crate::consensus::get_fork_choice_head(
            &self.chain,
            &justified_hash,
            &self.known_votes,
            0
        );
    }

    fn compute_safe_target(&self) -> Option<String> {
        let justified_hash = self.latest_justified_hash();
        let min_score = (self.num_validators * 2) / 3;
        
        let safe_target = crate::consensus::get_fork_choice_head(
            &self.chain,
            &justified_hash,
            &self.new_votes,
            min_score
        );
        
        Some(safe_target)
    }

    fn propose_block(&mut self) {
        let new_slot = self.get_current_slot();
        let head_state = self.post_states[&self.head].clone();
        
        let mut votes_to_add = Vec::new();
        
        // Keep attempting to add valid votes from the list of available votes
        loop {
            let new_block = Block {
                slot: new_slot,
                parent: Some(self.head.clone()),
                votes: votes_to_add.clone(),
                state_root: None,
            };
            
            let state = crate::consensus::process_block(head_state.clone(), &new_block);
            
            let new_votes_to_add: Vec<Vote> = self.known_votes.iter()
                .filter(|vote| {
                    vote.source == state.latest_justified_hash 
                    && !votes_to_add.contains(vote)
                })
                .cloned()
                .collect();
            
            if new_votes_to_add.is_empty() {
                break;
            }
            
            votes_to_add.extend(new_votes_to_add);
        }
        
        // Create final block with state root
        let mut new_block = Block {
            slot: new_slot,
            parent: Some(self.head.clone()),
            votes: votes_to_add,
            state_root: None,
        };
        
        let state = crate::consensus::process_block(head_state, &new_block);
        new_block.state_root = Some(compute_hash(&state));
        
        let new_hash = compute_hash(&new_block);
        self.chain.insert(new_hash.clone(), new_block.clone());
        self.post_states.insert(new_hash, state);
        
        // Submit to network
        self.network.borrow_mut().submit(Item::Block(new_block), self.validator_id);
    }

    fn vote(&mut self) {
        let state = self.post_states[&self.head].clone();
        let mut target_hash = self.head.clone();
        
        // If there is a safe target, use it; otherwise walk back 3 blocks
        let safe_target = self.safe_target.clone().unwrap_or_else(|| self.genesis_hash.clone());
        
        // Walk back 3 blocks from head if safe_target is not recent enough
        for _ in 0..3 {
            if let Some(block) = self.chain.get(&target_hash) {
                if block.slot > self.chain[&safe_target].slot {
                    if let Some(parent) = &block.parent {
                        target_hash = parent.clone();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Ensure the target slot is justifiable
        while let Some(target_block) = self.chain.get(&target_hash) {
            if crate::consensus::is_justifiable_slot(
                state.latest_finalized_slot,
                target_block.slot
            ) {
                break;
            }
            
            // Walk back to parent
            if let Some(parent) = &target_block.parent {
                target_hash = parent.clone();
            } else {
                break;
            }
        }
        
        let target_block = &self.chain[&target_hash];
        
        let vote = Vote {
            validator_id: self.validator_id,
            slot: self.get_current_slot(),
            head: self.head.clone(),
            head_slot: self.chain[&self.head].slot,
            target: target_hash.clone(),
            target_slot: target_block.slot,
            source: state.latest_justified_hash.clone(),
            source_slot: state.latest_justified_slot,
        };
        
        // Add to own known votes
        if !self.known_votes.contains(&vote) {
            self.known_votes.push(vote.clone());
        }
        
        // Submit to network
        self.network.borrow_mut().submit(Item::Vote(vote), self.validator_id);
    }

    fn accept_new_votes(&mut self) {
        for vote in self.new_votes.drain(..) {
            if !self.known_votes.contains(&vote) {
                self.known_votes.push(vote);
            }
        }
        self.recompute_head();
    }

    /// Executes protocol actions based on the current time within the slot
    /// 
    /// - t=0: Propose block (if validator's turn)
    /// - t=3: Vote
    /// - t=6: Compute safe target
    /// - t=9: Accept new votes
    pub fn tick(&mut self) {
        let time_in_slot = self.network.borrow().time % SLOT_DURATION;
        
        // t=0: propose a block
        if time_in_slot == 0 {
            if self.get_current_slot() % self.num_validators as u64 == self.validator_id {
                // View merge mechanism: accept attestations received <= 1/4 before slot start
                self.accept_new_votes();
                self.propose_block();
            }
        }
        // t=1/4: vote
        else if time_in_slot == SLOT_DURATION / 4 {
            self.vote();
        }
        // t=2/4: compute the safe target
        else if time_in_slot == (SLOT_DURATION * 2) / 4 {
            self.safe_target = self.compute_safe_target();
        }
        // Deadline to accept attestations except for those included in a block
        else if time_in_slot == (SLOT_DURATION * 3) / 4 {
            self.accept_new_votes();
        }
    }

    pub fn receive(&mut self, item: Item) {
        match item {
            Item::Block(block) => {
                let block_hash = compute_hash(&block);
                
                // If the block is already known, ignore it
                if self.chain.contains_key(&block_hash) {
                    return;
                }
                
                // Check if parent is known
                let parent_known = match &block.parent {
                    Some(parent) => self.post_states.contains_key(parent),
                    None => true, // Genesis block has no parent
                };
                
                if parent_known {
                    // Process block immediately
                    let parent_state = match &block.parent {
                        Some(parent) => self.post_states[parent].clone(),
                        None => {
                            // This shouldn't happen for non-genesis blocks
                            return;
                        }
                    };
                    
                    let state = crate::consensus::process_block(parent_state, &block);
                    self.chain.insert(block_hash.clone(), block.clone());
                    self.post_states.insert(block_hash.clone(), state);
                    
                    // Extract votes from the block
                    for vote in &block.votes {
                        if !self.known_votes.contains(vote) {
                            self.known_votes.push(vote.clone());
                        }
                    }
                    
                    self.recompute_head();
                    
                    // Process dependencies
                    if let Some(dependent_items) = self.dependencies.remove(&block_hash) {
                        for dep_item in dependent_items {
                            self.receive(dep_item);
                        }
                    }
                } else {
                    // Queue in dependencies
                    if let Some(parent) = &block.parent {
                        self.dependencies.entry(parent.clone())
                            .or_insert_with(Vec::new)
                            .push(Item::Block(block));
                    }
                }
            }
            Item::Vote(vote) => {
                // Check for duplicates
                if self.known_votes.contains(&vote) || self.new_votes.contains(&vote) {
                    return;
                }
                
                // Check if head is known
                if self.chain.contains_key(&vote.head) {
                    self.new_votes.push(vote);
                } else {
                    // Queue in dependencies
                    self.dependencies.entry(vote.head.clone())
                        .or_insert_with(Vec::new)
                        .push(Item::Vote(vote));
                }
            }
        }
    }
}

