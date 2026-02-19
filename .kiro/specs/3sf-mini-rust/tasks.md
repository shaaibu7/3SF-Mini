# Implementation Plan

- [x] 1. Fix and enhance existing types module
  - Update `Block` to use `Option<String>` for parent field to handle genesis block
  - Update `State` to use `Vec<Option<String>>` for historical_block_hashes to represent empty slots
  - Implement `PartialEq` and `Eq` for `Vote` to enable deduplication
  - Implement `Clone` for `Vote` and `Block`
  - Add `SLOT_DURATION` constant (value: 12)
  - _Requirements: 1.1, 3.1, 3.2, 6.3_

- [ ]* 1.1 Write property test for hash determinism
  - **Property 1: Hash determinism**
  - **Validates: Requirements 1.1, 1.2, 1.3**

- [x] 2. Complete consensus module implementation
  - Fix `is_pronic` function to correctly detect oblong numbers (x² + x)
  - Make `get_latest_justified_hash` public
  - Update `process_block` to take `&Block` instead of `Block` for efficiency
  - Fix bug in `process_block` where it uses `state.latest_finalized_slot` instead of `state_copy.latest_finalized_slot` in finalization check
  - _Requirements: 2.1, 2.2, 2.3, 3.1-3.8_

- [ ]* 2.1 Write property tests for justifiable slots
  - **Property 2: Recent slots are justifiable**
  - **Validates: Requirements 2.1**

- [ ]* 2.2 Write property test for perfect squares
  - **Property 3: Perfect square deltas are justifiable**
  - **Validates: Requirements 2.2**

- [ ]* 2.3 Write property test for oblong numbers
  - **Property 4: Oblong deltas are justifiable**
  - **Validates: Requirements 2.3**

- [ ]* 2.4 Write property test for invalid slot panic
  - **Property 5: Invalid slots cause panic**
  - **Validates: Requirements 2.4**

- [ ]* 2.5 Write property test for block processing
  - **Property 6: Block processing appends parent and pads history**
  - **Validates: Requirements 3.1, 3.2**

- [ ]* 2.6 Write property test for invalid vote handling
  - **Property 7: Invalid votes are ignored**
  - **Validates: Requirements 3.3, 3.4, 3.5, 3.6**

- [ ]* 2.7 Write property test for justification threshold
  - **Property 8: Justification at 2/3 threshold**
  - **Validates: Requirements 3.7**

- [ ]* 2.8 Write property test for finalization
  - **Property 9: Finalization without intermediate justifiable slots**
  - **Validates: Requirements 3.8**

- [x] 3. Implement fork choice (LMD GHOST) algorithm
  - Implement `get_fork_choice_head` function with signature: `pub fn get_fork_choice_head(blocks: &HashMap<String, Block>, root: &str, votes: &[Vote], min_score: usize) -> String`
  - Extract latest vote from each validator
  - Build vote weight map by counting votes for each block and its ancestors
  - Build children map from block parent relationships
  - Implement greedy heaviest subtree selection with tiebreaking (vote weight, then slot, then hash)
  - Handle ZERO_HASH root by finding genesis block (minimum slot)
  - _Requirements: 5.1-5.7_

- [ ]* 3.1 Write property test for latest justified hash
  - **Property 10: Latest justified hash has maximum slot**
  - **Validates: Requirements 4.1**

- [ ]* 3.2 Write property test for latest vote counting
  - **Property 11: Fork choice uses latest votes only**
  - **Validates: Requirements 5.2**

- [ ]* 3.3 Write property test for vote propagation
  - **Property 12: Fork choice counts descendant votes**
  - **Validates: Requirements 5.1, 5.3**

- [ ]* 3.4 Write property test for heaviest child selection
  - **Property 13: Fork choice selects heaviest child**
  - **Validates: Requirements 5.4**

- [ ]* 3.5 Write property test for fork choice tiebreaking
  - **Property 14: Fork choice tiebreaking is deterministic**
  - **Validates: Requirements 5.5**

- [ ]* 3.6 Write property test for leaf termination
  - **Property 15: Fork choice terminates at leaf**
  - **Validates: Requirements 5.6**

- [ ]* 3.7 Write property test for minimum score filtering
  - **Property 16: Fork choice respects minimum score**
  - **Validates: Requirements 5.7**

- [ ] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Create P2P module with network simulator
  - Create `src/p2p.rs` file
  - Define `Item` enum with `Block(Block)` and `Vote(Vote)` variants
  - Implement `P2PNetwork` struct with fields: time, stakers, queues, latency_func
  - Implement `P2PNetwork::new(latency_func)` constructor
  - Implement `P2PNetwork::register_staker()` method
  - Implement `P2PNetwork::submit()` method to schedule message delivery
  - Implement `P2PNetwork::time_step()` method to advance time and deliver messages
  - Add `pub mod p2p;` to `src/lib.rs`
  - _Requirements: 12.1-12.5, 13.1, 14.1, 14.2_

- [ ]* 5.1 Write property test for message broadcasting
  - **Property 39: Messages broadcast to all others**
  - **Validates: Requirements 12.1, 12.5**

- [ ]* 5.2 Write property test for latency application
  - **Property 40: Message delivery uses latency function**
  - **Validates: Requirements 12.2**

- [ ]* 5.3 Write property test for message delivery
  - **Property 41: Time advancement delivers messages**
  - **Validates: Requirements 12.3, 14.2**

- [ ]* 5.4 Write property test for time increment
  - **Property 45: Time increments**
  - **Validates: Requirements 14.1**

- [x] 6. Implement Staker node structure and initialization
  - Define `Staker` struct with fields: validator_id, network, chain, post_states, known_votes, new_votes, dependencies, genesis_hash, num_validators, safe_target, head
  - Use `Rc<RefCell<P2PNetwork>>` for network field to enable shared mutable access
  - Implement `Staker::new()` constructor that accepts validator_id, network, genesis_block, genesis_state
  - Initialize chain and post_states with genesis block
  - Register staker with network
  - Implement helper methods: `latest_justified_hash()`, `latest_finalized_hash()`, `get_current_slot()`
  - _Requirements: 13.2-13.5_

- [ ]* 6.1 Write property test for staker registration
  - **Property 43: Staker registration**
  - **Validates: Requirements 13.3**

- [ ]* 6.2 Write property test for genesis storage
  - **Property 44: Genesis storage**
  - **Validates: Requirements 13.4**

- [x] 7. Implement Staker receive method for blocks and votes
  - Implement `Staker::receive(&mut self, item: Item)` method
  - Handle `Item::Block`: check if parent is known, process immediately or queue in dependencies
  - When processing block: compute hash, call `process_block`, store in chain and post_states, extract votes, recompute head
  - Handle dependency resolution: recursively process dependent items when parent/head arrives
  - Handle `Item::Vote`: check if head is known, add to new_votes or queue in dependencies
  - Implement deduplication for votes (check known_votes and new_votes)
  - _Requirements: 6.1-6.6_

- [ ]* 7.1 Write property test for unknown parent queuing
  - **Property 17: Unknown parent blocks are queued**
  - **Validates: Requirements 6.1**

- [ ]* 7.2 Write property test for known parent processing
  - **Property 18: Known parent blocks are processed immediately**
  - **Validates: Requirements 6.2**

- [ ]* 7.3 Write property test for vote extraction
  - **Property 19: Block votes are extracted**
  - **Validates: Requirements 6.3**

- [ ]* 7.4 Write property test for unknown head vote queuing
  - **Property 20: Unknown head votes are queued**
  - **Validates: Requirements 6.4**

- [ ]* 7.5 Write property test for known head vote acceptance
  - **Property 21: Known head votes are accepted**
  - **Validates: Requirements 6.5**

- [ ]* 7.6 Write property test for dependency resolution
  - **Property 22: Dependencies are resolved recursively**
  - **Validates: Requirements 6.6**

- [ ]* 7.7 Write property test for message processing
  - **Property 42: Delivered messages invoke receive**
  - **Validates: Requirements 12.4**

- [x] 8. Implement Staker fork choice and safe target methods
  - Implement `Staker::recompute_head(&mut self)` method using `get_fork_choice_head`
  - Implement `Staker::compute_safe_target(&self) -> Option<String>` method
  - Use fork choice with `min_score = (num_validators * 2) / 3`
  - Start from latest justified hash
  - _Requirements: 7.1-7.3_

- [ ]* 8.1 Write property test for safe target supermajority
  - **Property 23: Safe target has supermajority support**
  - **Validates: Requirements 7.3**

- [x] 9. Implement Staker block proposal logic
  - Implement `Staker::propose_block(&mut self)` method
  - Create block with slot = `get_current_slot()`, parent = head
  - Iteratively add valid votes from known_votes (votes with source = latest_justified_hash)
  - Process block to get new state, compute state_root
  - Store block in chain and post_states
  - Submit block to network
  - _Requirements: 8.1-8.6_

- [ ]* 9.1 Write property test for proposal parent
  - **Property 25: Proposed blocks have correct parent**
  - **Validates: Requirements 8.2**

- [ ]* 9.2 Write property test for vote inclusion
  - **Property 26: Proposed blocks include valid votes**
  - **Validates: Requirements 8.3, 8.4**

- [ ]* 9.3 Write property test for state root
  - **Property 27: Proposed blocks have state root**
  - **Validates: Requirements 8.5**

- [ ]* 9.4 Write property test for block broadcast
  - **Property 28: Proposed blocks are broadcast**
  - **Validates: Requirements 8.6**

- [x] 10. Implement Staker voting logic
  - Implement `Staker::vote(&mut self)` method
  - Get current head and state
  - Select target: use safe_target if available, otherwise walk back 3 blocks from head
  - Ensure target slot is justifiable (walk back further if needed)
  - Create vote with head, target, source (latest justified hash and slot)
  - Add vote to own known_votes
  - Submit vote to network
  - _Requirements: 9.1-9.7_

- [ ]* 10.1 Write property test for vote head reference
  - **Property 30: Votes reference current head**
  - **Validates: Requirements 9.2**

- [ ]* 10.2 Write property test for target ancestry
  - **Property 31: Vote targets are ancestors of head**
  - **Validates: Requirements 9.3**

- [ ]* 10.3 Write property test for target justifiability
  - **Property 32: Vote targets are justifiable**
  - **Validates: Requirements 9.5**

- [ ]* 10.4 Write property test for vote source
  - **Property 33: Vote sources are latest justified**
  - **Validates: Requirements 9.6**

- [ ]* 10.5 Write property test for vote self-tracking
  - **Property 34: Votes are self-tracked**
  - **Validates: Requirements 9.7**

- [x] 11. Implement Staker view merge and timing logic
  - Implement `Staker::accept_new_votes(&mut self)` method
  - Move votes from new_votes to known_votes (with deduplication)
  - Call recompute_head after accepting votes
  - Implement `Staker::tick(&mut self)` method
  - Calculate `time_in_slot = network.time % SLOT_DURATION`
  - At t=0: if validator's turn, accept_new_votes then propose_block
  - At t=SLOT_DURATION/4: vote
  - At t=SLOT_DURATION/2: compute and store safe_target
  - At t=3*SLOT_DURATION/4: accept_new_votes
  - _Requirements: 9.1, 10.1-10.4, 11.1, 14.3, 14.4_

- [ ]* 11.1 Write property test for proposal timing
  - **Property 24: Proposals at correct time**
  - **Validates: Requirements 8.1**

- [ ]* 11.2 Write property test for voting timing
  - **Property 29: Votes at correct time**
  - **Validates: Requirements 9.1**

- [ ]* 11.3 Write property test for vote acceptance at proposal
  - **Property 35: Vote acceptance at proposal time**
  - **Validates: Requirements 10.1**

- [ ]* 11.4 Write property test for vote acceptance at deadline
  - **Property 36: Vote acceptance at deadline**
  - **Validates: Requirements 10.2**

- [ ]* 11.5 Write property test for vote movement
  - **Property 37: Vote acceptance moves votes**
  - **Validates: Requirements 10.3**

- [ ]* 11.6 Write property test for safe target timing
  - **Property 38: Safe target computation timing**
  - **Validates: Requirements 11.1**

- [ ]* 11.7 Write property test for tick dispatch
  - **Property 46: Tick dispatches correctly**
  - **Validates: Requirements 14.3**

- [x] 12. Create example simulation in main.rs
  - Create genesis block and state
  - Initialize P2P network with simple latency function (e.g., constant 1-2 time units)
  - Create 4 staker nodes
  - Run simulation loop: call time_step on network, then tick on each staker
  - Print progress: show current time, slot, justified/finalized blocks
  - Run for enough time to observe justification and finalization (e.g., 100 time units)
  - _Requirements: 13.1-13.5, 14.1-14.4_

- [ ] 13. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 14. Update documentation
  - Update README.md with project description, build instructions, and usage examples
  - Add inline documentation comments to public functions and structs
  - Include explanation of 3SF-mini protocol and link to reference implementation
  - _Requirements: All_
