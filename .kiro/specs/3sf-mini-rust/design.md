# Design Document: 3SF-Mini Rust Implementation

## Overview

This design document describes the architecture and implementation approach for 3SF-mini (Three Slot Finality - Minimal) in Rust. The system implements a simplified consensus protocol that achieves finality within three slots under normal network conditions. The implementation consists of three main components: consensus logic, fork choice (LMD GHOST), and P2P network simulation.

The design follows the Python reference implementation from the Ethereum research repository while leveraging Rust's type safety, ownership model, and performance characteristics.

## Architecture

The system is organized into three primary modules:

1. **Types Module** (`types.rs`): Defines core data structures including Config, State, Vote, and Block
2. **Consensus Module** (`consensus.rs`): Implements consensus logic including block processing, justification, finalization, and fork choice
3. **P2P Module** (`p2p.rs`): Implements the network simulator and staker node logic

### Component Interaction Flow

```
┌─────────────┐
│  P2P Network│
│  Simulator  │
└──────┬──────┘
       │ time_step()
       ├──────────────┐
       │              │
       v              v
┌──────────┐    ┌──────────┐
│ Staker 1 │    │ Staker 2 │
│          │◄───┤          │
│  tick()  │    │  tick()  │
└────┬─────┘    └────┬─────┘
     │               │
     │ propose/vote  │
     v               v
┌─────────────────────────┐
│   Consensus Logic       │
│  - process_block()      │
│  - get_fork_choice()    │
│  - is_justifiable_slot()│
└─────────────────────────┘
```

## Components and Interfaces

### Types Module

#### Config
```rust
pub struct Config {
    pub num_validators: u64
}
```
Stores the number of validators in the network.

#### State
```rust
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
```
Tracks the consensus state including justified and finalized blocks, historical hashes, and ongoing justification attempts.

**Key Design Decision**: Use `Vec<Option<String>>` for `historical_block_hashes` to represent empty slots (None) vs blocks (Some(hash)). This differs from the Python implementation which uses None directly.

#### Vote
```rust
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
```
Represents a validator's attestation with head (current view), target (block to justify), and source (previously justified block).

**Key Design Decision**: Implement `PartialEq` and `Eq` for Vote to enable deduplication in vote lists.

#### Block
```rust
pub struct Block {
    pub slot: u64,
    pub parent: Option<String>,
    pub votes: Vec<Vote>,
    pub state_root: Option<String>
}
```
Represents a block in the chain.

**Key Design Decision**: Use `Option<String>` for parent and state_root to handle genesis block (no parent) and blocks before state computation.

### Consensus Module

#### Hash Computation
```rust
pub fn compute_hash<T: Serialize>(obj: &T) -> String
```
Serializes objects to deterministic JSON and computes SHA-256 hash. This ensures all nodes agree on object identifiers.

#### Justifiable Slot Logic
```rust
pub fn is_justifiable_slot(finalized_slot: u64, candidate: u64) -> bool
```
Determines if a slot is eligible for justification based on:
- Delta ≤ 5 (recent slots)
- Delta is a perfect square (x²)
- Delta is an oblong number (x² + x)

This backoff mechanism ensures finality progresses even under high latency.

#### Block Processing
```rust
pub fn process_block(state: State, block: &Block) -> State
```
Core consensus logic that:
1. Updates historical block hashes
2. Processes votes from the block
3. Tracks justification attempts
4. Justifies blocks when 2/3 threshold is reached
5. Finalizes blocks when appropriate

**Key Design Decision**: Take state by value and return new state to maintain immutability and avoid shared mutable state issues.

#### Latest Justified Hash
```rust
pub fn get_latest_justified_hash(post_states: &HashMap<String, State>) -> String
```
Finds the justified block with the highest slot across all known chain branches.

#### Fork Choice (LMD GHOST)
```rust
pub fn get_fork_choice_head(
    blocks: &HashMap<String, Block>,
    root: &str,
    votes: &[Vote],
    min_score: usize
) -> String
```
Implements Latest Message Driven Greedy Heaviest Observed SubTree:
1. Identifies latest vote from each validator
2. Counts vote weights for each block (including descendant votes)
3. Starts at root and repeatedly chooses heaviest child
4. Tiebreaks by slot then hash

**Key Design Decision**: Use `min_score` parameter to enable safe target computation (requiring 2/3 support).

### P2P Module

#### Staker Node
```rust
pub struct Staker {
    validator_id: u64,
    network: Rc<RefCell<P2PNetwork>>,
    chain: HashMap<String, Block>,
    post_states: HashMap<String, State>,
    known_votes: Vec<Vote>,
    new_votes: Vec<Vote>,
    dependencies: HashMap<String, Vec<Item>>,
    genesis_hash: String,
    num_validators: usize,
    safe_target: Option<String>,
    head: String
}
```

**Key Design Decision**: Use `Rc<RefCell<P2PNetwork>>` to allow shared mutable access to the network from multiple stakers. This is necessary because stakers need to submit messages to the network.

**Methods**:
- `new()`: Initialize with genesis block and state
- `receive()`: Process incoming blocks and votes
- `tick()`: Execute time-based protocol actions
- `propose_block()`: Create and broadcast new block
- `vote()`: Create and broadcast vote
- `accept_new_votes()`: Move new votes to known votes (view merge)
- `compute_safe_target()`: Find block with 2/3 support
- `recompute_head()`: Update head using fork choice

#### P2P Network
```rust
pub struct P2PNetwork {
    time: u64,
    stakers: HashMap<u64, Rc<RefCell<Staker>>>,
    queues: HashMap<u64, Vec<(u64, Item)>>,
    latency_func: Box<dyn Fn(u64) -> u64>
}

pub enum Item {
    Block(Block),
    Vote(Vote)
}
```

**Key Design Decision**: Use `Box<dyn Fn(u64) -> u64>` for latency function to allow flexible network conditions. Use enum for messages to handle both blocks and votes uniformly.

**Methods**:
- `new()`: Initialize with latency function
- `register_staker()`: Add staker to network
- `submit()`: Schedule message delivery to all other stakers
- `time_step()`: Advance time and deliver pending messages

## Data Models

### Slot Timing Model

Each slot has duration of 12 time units with specific protocol phases:
- **t=0**: Block proposal (if validator's turn)
- **t=3** (SLOT_DURATION/4): Voting
- **t=6** (SLOT_DURATION/2): Safe target computation
- **t=9** (3*SLOT_DURATION/4): Vote acceptance deadline

### Justification and Finalization Model

**Justification**: A block becomes justified when 2/3 of validators vote for it as their target, with votes sourcing from an already-justified block.

**Finalization**: When a block B is justified, its source block S becomes finalized if there are no intermediate justifiable slots between S and B.

### Dependency Management

Blocks and votes may arrive out of order. The staker maintains a dependency map:
```
dependencies: HashMap<String, Vec<Item>>
```
When a block/vote arrives referencing an unknown parent/head, it's stored under that hash. When the parent/head arrives, all dependencies are recursively processed.

## Error Handling

### Consensus Errors
- **Invalid slot**: Assert that candidate slot ≥ finalized slot in `is_justifiable_slot`
- **Empty post_states**: Panic in `get_latest_justified_hash` if no states exist (should never happen in valid usage)
- **Unknown genesis**: Panic if genesis block not found when root is ZERO_HASH

### Network Errors
- **Missing staker**: Gracefully skip if staker not found during message delivery
- **Serialization errors**: Unwrap serialization (should never fail for valid types)

### Design Decision
Use panics for programmer errors (invalid state) and graceful handling for network conditions (missing data). This follows Rust conventions.

## Testing Strategy

### Unit Testing Framework
Use Rust's built-in test framework with `#[cfg(test)]` modules.

### Property-Based Testing Framework
Use **proptest** crate for property-based testing. Configure tests to run minimum 100 iterations.

### Test Organization
- Unit tests co-located with implementation in each module
- Property tests in separate test files under `tests/` directory
- Integration tests for full protocol scenarios

### Unit Test Coverage
- Hash computation produces consistent outputs
- Justifiable slot logic for edge cases (delta=0, delta=5, delta=6, perfect squares, oblongs)
- Block processing with empty votes
- Block processing with invalid votes
- Fork choice with single chain
- Fork choice with simple fork
- Dependency resolution when parent arrives

### Property-Based Test Coverage
Property-based tests will be defined after prework analysis in the Correctness Properties section below.


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

After analyzing the acceptance criteria, I've identified properties that can be consolidated to eliminate redundancy. Several properties test the same underlying behavior from different angles, and some can be combined into more comprehensive tests.

### Hash and Serialization Properties

Property 1: Hash determinism
*For any* consensus object (Block, Vote, State, Config), hashing it multiple times should produce identical outputs
**Validates: Requirements 1.1, 1.2, 1.3**

### Justifiable Slot Properties

Property 2: Recent slots are justifiable
*For any* finalized slot and candidate slot where delta ≤ 5, the candidate should be marked as justifiable
**Validates: Requirements 2.1**

Property 3: Perfect square deltas are justifiable
*For any* finalized slot and perfect square delta (x²), the candidate slot (finalized + delta) should be marked as justifiable
**Validates: Requirements 2.2**

Property 4: Oblong deltas are justifiable
*For any* finalized slot and oblong delta (x² + x), the candidate slot (finalized + delta) should be marked as justifiable
**Validates: Requirements 2.3**

Property 5: Invalid slots cause panic
*For any* candidate slot less than finalized slot, calling is_justifiable_slot should panic
**Validates: Requirements 2.4**

### Block Processing Properties

Property 6: Block processing appends parent and pads history
*For any* valid state and block, processing the block should result in historical_block_hashes containing the parent and having length equal to the block's slot number
**Validates: Requirements 3.1, 3.2**

Property 7: Invalid votes are ignored
*For any* state and block containing votes with unjustified sources, invalid targets, target_slot ≤ source_slot, or non-justifiable target slots, processing the block should not create new justification entries for those votes
**Validates: Requirements 3.3, 3.4, 3.5, 3.6**

Property 8: Justification at 2/3 threshold
*For any* valid state, target hash, and set of exactly 2/3 validators voting for that target, processing a block with those votes should mark the target as justified
**Validates: Requirements 3.7**

Property 9: Finalization without intermediate justifiable slots
*For any* state where a target is justified and there are no intermediate justifiable slots between source and target, the source block should be finalized
**Validates: Requirements 3.8**

### Fork Choice Properties

Property 10: Latest justified hash has maximum slot
*For any* non-empty collection of post-states, the latest justified hash should have a justified slot greater than or equal to all other justified slots in the collection
**Validates: Requirements 4.1**

Property 11: Fork choice uses latest votes only
*For any* block tree, root, and set of votes where a validator has multiple votes, the fork choice should only count the vote with the highest slot number from that validator
**Validates: Requirements 5.2**

Property 12: Fork choice counts descendant votes
*For any* block tree, root, and vote set, if a vote is for block B, then all ancestors of B (up to the root) should have their vote weight increased
**Validates: Requirements 5.1, 5.3**

Property 13: Fork choice selects heaviest child
*For any* block with multiple children having different vote weights, fork choice should select the child with the highest vote weight
**Validates: Requirements 5.4**

Property 14: Fork choice tiebreaking is deterministic
*For any* block with multiple children having equal vote weights, fork choice should select the child with the highest slot, and if slots are equal, the lexicographically greatest hash
**Validates: Requirements 5.5**

Property 15: Fork choice terminates at leaf
*For any* linear chain from root to leaf, fork choice should return the leaf block
**Validates: Requirements 5.6**

Property 16: Fork choice respects minimum score
*For any* block tree, root, vote set, and minimum score threshold, fork choice should only consider blocks with vote weight ≥ min_score
**Validates: Requirements 5.7**

### Staker Node Properties

Property 17: Unknown parent blocks are queued
*For any* staker and block with unknown parent, receiving the block should add it to the dependencies map under the parent hash, not to the chain
**Validates: Requirements 6.1**

Property 18: Known parent blocks are processed immediately
*For any* staker and block with known parent, receiving the block should add it to both chain and post_states
**Validates: Requirements 6.2**

Property 19: Block votes are extracted
*For any* staker and block with votes, processing the block should add all votes to known_votes
**Validates: Requirements 6.3**

Property 20: Unknown head votes are queued
*For any* staker and vote with unknown head block, receiving the vote should add it to dependencies under the head hash
**Validates: Requirements 6.4**

Property 21: Known head votes are accepted
*For any* staker and vote with known head block, receiving the vote should add it to new_votes
**Validates: Requirements 6.5**

Property 22: Dependencies are resolved recursively
*For any* staker with queued dependencies, when the dependency key (parent/head) is received, all dependent items should be processed and removed from the dependency queue
**Validates: Requirements 6.6**

Property 23: Safe target has supermajority support
*For any* staker that computes a safe target, the resulting block should have vote weight ≥ 2/3 of validators
**Validates: Requirements 7.3**

### Block Proposal Properties

Property 24: Proposals at correct time
*For any* staker at time t where t % SLOT_DURATION == 0 and (t / SLOT_DURATION + 2) % num_validators == validator_id, calling tick should result in a new block being proposed
**Validates: Requirements 8.1**

Property 25: Proposed blocks have correct parent
*For any* proposed block, the parent should equal the staker's current head
**Validates: Requirements 8.2**

Property 26: Proposed blocks include valid votes
*For any* proposed block, all included votes should have source matching the latest justified hash at proposal time
**Validates: Requirements 8.3, 8.4**

Property 27: Proposed blocks have state root
*For any* proposed block, the state_root field should be Some(hash), not None
**Validates: Requirements 8.5**

Property 28: Proposed blocks are broadcast
*For any* staker that proposes a block, the block should appear in the network's message queues for all other validators
**Validates: Requirements 8.6**

### Voting Properties

Property 29: Votes at correct time
*For any* staker at time t where t % SLOT_DURATION == SLOT_DURATION/4, calling tick should result in a new vote being created
**Validates: Requirements 9.1**

Property 30: Votes reference current head
*For any* created vote, the head field should equal the staker's current head at vote creation time
**Validates: Requirements 9.2**

Property 31: Vote targets are ancestors of head
*For any* created vote, the target block should be an ancestor of the head block (reachable by following parent links)
**Validates: Requirements 9.3**

Property 32: Vote targets are justifiable
*For any* created vote, is_justifiable_slot(latest_finalized_slot, target_slot) should return true
**Validates: Requirements 9.5**

Property 33: Vote sources are latest justified
*For any* created vote, the source field should equal the latest justified hash at vote creation time
**Validates: Requirements 9.6**

Property 34: Votes are self-tracked
*For any* staker that creates a vote, the vote should be added to the staker's known_votes list
**Validates: Requirements 9.7**

### View Merge Properties

Property 35: Vote acceptance at proposal time
*For any* staker at time t where t % SLOT_DURATION == 0 and it's the staker's turn to propose, calling tick should move all new_votes to known_votes
**Validates: Requirements 10.1**

Property 36: Vote acceptance at deadline
*For any* staker at time t where t % SLOT_DURATION == 3*SLOT_DURATION/4, calling tick should move all new_votes to known_votes
**Validates: Requirements 10.2**

Property 37: Vote acceptance moves votes
*For any* staker with non-empty new_votes, calling accept_new_votes should result in new_votes being empty and all votes being in known_votes
**Validates: Requirements 10.3**

Property 38: Safe target computation timing
*For any* staker at time t where t % SLOT_DURATION == SLOT_DURATION/2, calling tick should update the safe_target field
**Validates: Requirements 11.1**

### P2P Network Properties

Property 39: Messages broadcast to all others
*For any* staker that submits a message, the message should be scheduled in the network queues for all validators except the sender
**Validates: Requirements 12.1, 12.5**

Property 40: Message delivery uses latency function
*For any* message submitted at time T, the scheduled delivery time should equal T + latency_func(T)
**Validates: Requirements 12.2**

Property 41: Time advancement delivers messages
*For any* network with queued messages, calling time_step should deliver all messages with delivery_time ≤ current_time and remove them from queues
**Validates: Requirements 12.3, 14.2**

Property 42: Delivered messages invoke receive
*For any* message delivered to a staker, the staker's state should reflect that the message was processed (block in chain or vote in new_votes)
**Validates: Requirements 12.4**

Property 43: Staker registration
*For any* staker that initializes with a network, the staker should be present in the network's stakers map
**Validates: Requirements 13.3**

Property 44: Genesis storage
*For any* staker that initializes with genesis block and state, both chain and post_states should contain entries for the genesis hash
**Validates: Requirements 13.4**

Property 45: Time increments
*For any* network, calling time_step should increase the time field by exactly 1
**Validates: Requirements 14.1**

Property 46: Tick dispatches correctly
*For any* staker and time t, calling tick should execute the action corresponding to t % SLOT_DURATION (propose at 0, vote at 3, safe target at 6, accept votes at 9)
**Validates: Requirements 14.3**

