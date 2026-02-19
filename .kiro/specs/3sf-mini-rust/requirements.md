# Requirements Document

## Introduction

This document specifies the requirements for implementing 3SF-mini (Three Slot Finality - Minimal) in Rust. 3SF-mini is a simplified consensus protocol that achieves finality within three slots under normal network conditions. The implementation is based on the Python reference implementation from the Ethereum research repository and includes consensus logic, fork choice (LMD GHOST), and P2P network simulation capabilities.

## Glossary

- **Consensus System**: The 3SF-mini blockchain consensus implementation
- **Staker Node**: A validator node that participates in consensus by proposing blocks and voting
- **P2P Network**: The peer-to-peer network simulator that manages message delivery between staker nodes
- **Fork Choice**: The LMD GHOST algorithm used to determine the canonical chain head
- **Vote**: An attestation from a validator containing head, target, and source information
- **Block**: A data structure containing a slot number, parent hash, votes, and state root
- **State**: The consensus state tracking justified and finalized blocks
- **Justification**: The process of marking a block as justified when 2/3 of validators vote for it
- **Finalization**: The process of marking a block as finalized when justification occurs without intermediate justifiable slots
- **Justifiable Slot**: A slot that is eligible for justification based on distance from the latest finalized slot

## Requirements

### Requirement 1

**User Story:** As a blockchain developer, I want to serialize and hash consensus objects deterministically, so that all nodes can agree on block and state identifiers.

#### Acceptance Criteria

1. WHEN the Consensus System serializes any consensus object THEN the Consensus System SHALL produce a deterministic JSON representation with sorted keys
2. WHEN the Consensus System hashes a serialized object THEN the Consensus System SHALL compute a SHA-256 hash and return it as a hexadecimal string
3. WHEN the Consensus System hashes identical objects multiple times THEN the Consensus System SHALL produce identical hash outputs

### Requirement 2

**User Story:** As a consensus protocol designer, I want to determine which slots are eligible for justification, so that the protocol can achieve finality even under high latency conditions.

#### Acceptance Criteria

1. WHEN the delta between a candidate slot and the latest finalized slot is less than or equal to 5 THEN the Consensus System SHALL mark the candidate slot as justifiable
2. WHEN the delta is a perfect square (x²) THEN the Consensus System SHALL mark the candidate slot as justifiable
3. WHEN the delta is an oblong number (x² + x) THEN the Consensus System SHALL mark the candidate slot as justifiable
4. WHEN the candidate slot is before the finalized slot THEN the Consensus System SHALL reject the operation with an assertion failure

### Requirement 3

**User Story:** As a validator node, I want to process blocks and update consensus state, so that the blockchain can progress toward finality.

#### Acceptance Criteria

1. WHEN the Consensus System processes a block THEN the Consensus System SHALL append the block's parent hash to the historical block hashes list
2. WHEN the historical block hashes list is shorter than the block's slot number THEN the Consensus System SHALL pad the list with null entries
3. WHEN the Consensus System processes a vote with an unjustified source THEN the Consensus System SHALL ignore the vote
4. WHEN the Consensus System processes a vote with a target not in historical hashes THEN the Consensus System SHALL ignore the vote
5. WHEN the Consensus System processes a vote with a target slot less than or equal to source slot THEN the Consensus System SHALL ignore the vote
6. WHEN the Consensus System processes a vote for a non-justifiable target slot THEN the Consensus System SHALL ignore the vote
7. WHEN the Consensus System receives 2/3 of validators voting for the same target THEN the Consensus System SHALL mark that target as justified
8. WHEN the Consensus System justifies a target with no intermediate justifiable slots after the source THEN the Consensus System SHALL finalize the source block

### Requirement 4

**User Story:** As a validator node, I want to determine the latest justified block across all known chain branches, so that I can build on the most secure foundation.

#### Acceptance Criteria

1. WHEN the Consensus System queries for the latest justified hash across multiple post-states THEN the Consensus System SHALL return the justified hash with the highest slot number
2. WHEN multiple post-states have the same highest justified slot THEN the Consensus System SHALL return any of the tied justified hashes

### Requirement 5

**User Story:** As a validator node, I want to use LMD GHOST fork choice to determine the canonical chain head, so that the network converges on a single chain.

#### Acceptance Criteria

1. WHEN the Fork Choice algorithm starts from a root block THEN the Fork Choice algorithm SHALL count votes for all descendants of that root
2. WHEN the Fork Choice algorithm counts votes THEN the Fork Choice algorithm SHALL use only the latest vote from each validator
3. WHEN the Fork Choice algorithm counts votes for a block THEN the Fork Choice algorithm SHALL include votes for any descendant of that block
4. WHEN the Fork Choice algorithm selects among children THEN the Fork Choice algorithm SHALL choose the child with the highest vote weight
5. WHEN multiple children have equal vote weights THEN the Fork Choice algorithm SHALL tiebreak by highest slot number then by hash lexicographically
6. WHEN the Fork Choice algorithm reaches a block with no children THEN the Fork Choice algorithm SHALL return that block as the head
7. WHEN a minimum score threshold is specified THEN the Fork Choice algorithm SHALL exclude blocks with vote weights below that threshold from consideration

### Requirement 6

**User Story:** As a staker node, I want to maintain local blockchain state and process incoming blocks and votes, so that I can participate in consensus.

#### Acceptance Criteria

1. WHEN the Staker Node receives a block with an unknown parent THEN the Staker Node SHALL store the block in a dependency queue until the parent arrives
2. WHEN the Staker Node receives a block with a known parent THEN the Staker Node SHALL process the block immediately and update the post-state
3. WHEN the Staker Node processes a block THEN the Staker Node SHALL extract and track all votes contained in that block
4. WHEN the Staker Node receives a vote for an unknown head block THEN the Staker Node SHALL store the vote in a dependency queue until the head block arrives
5. WHEN the Staker Node receives a vote for a known head block THEN the Staker Node SHALL add the vote to the new votes list
6. WHEN the Staker Node processes a block with dependencies THEN the Staker Node SHALL recursively process all dependent blocks and votes

### Requirement 7

**User Story:** As a staker node, I want to compute a safe target block for voting, so that I follow the view merge mechanism correctly.

#### Acceptance Criteria

1. WHEN the Staker Node computes a safe target THEN the Staker Node SHALL start from the latest justified hash
2. WHEN the Staker Node computes a safe target THEN the Staker Node SHALL use fork choice with a minimum score of 2/3 of validators
3. WHEN the Staker Node computes a safe target THEN the Staker Node SHALL return the block that has supermajority support

### Requirement 8

**User Story:** As a staker node, I want to propose blocks at designated times, so that the blockchain progresses through slots.

#### Acceptance Criteria

1. WHEN the network time reaches the start of a slot (t=0) and it is the Staker Node's turn THEN the Staker Node SHALL propose a new block
2. WHEN the Staker Node proposes a block THEN the Staker Node SHALL set the parent to the current head
3. WHEN the Staker Node proposes a block THEN the Staker Node SHALL include all valid votes from the known votes list
4. WHEN the Staker Node includes votes in a block THEN the Staker Node SHALL only include votes whose source matches the latest justified hash
5. WHEN the Staker Node proposes a block THEN the Staker Node SHALL compute and set the state root
6. WHEN the Staker Node proposes a block THEN the Staker Node SHALL broadcast the block to the P2P Network

### Requirement 9

**User Story:** As a staker node, I want to vote at designated times, so that I contribute to justification and finalization.

#### Acceptance Criteria

1. WHEN the network time reaches 1/4 through a slot (t=SLOT_DURATION/4) THEN the Staker Node SHALL create and broadcast a vote
2. WHEN the Staker Node creates a vote THEN the Staker Node SHALL set the head to the current fork choice head
3. WHEN the Staker Node creates a vote THEN the Staker Node SHALL set the target to a safe ancestor of the head
4. WHEN the Staker Node selects a target THEN the Staker Node SHALL use the safe target if available, otherwise use an ancestor 3 blocks back
5. WHEN the Staker Node selects a target THEN the Staker Node SHALL ensure the target slot is justifiable
6. WHEN the Staker Node creates a vote THEN the Staker Node SHALL set the source to the latest justified hash and slot
7. WHEN the Staker Node creates a vote THEN the Staker Node SHALL include the vote in its own known votes list

### Requirement 10

**User Story:** As a staker node, I want to accept new votes at specific times according to the view merge mechanism, so that the protocol maintains safety.

#### Acceptance Criteria

1. WHEN the network time reaches t=0 and it is the Staker Node's turn to propose THEN the Staker Node SHALL accept new votes received up to 1/4 slot before
2. WHEN the network time reaches 3/4 through a slot (t=3*SLOT_DURATION/4) THEN the Staker Node SHALL accept all new votes received
3. WHEN the Staker Node accepts new votes THEN the Staker Node SHALL move votes from the new votes list to the known votes list
4. WHEN the Staker Node accepts new votes THEN the Staker Node SHALL recompute the fork choice head

### Requirement 11

**User Story:** As a staker node, I want to compute the safe target at a specific time, so that all honest nodes converge on the same target under network assumptions.

#### Acceptance Criteria

1. WHEN the network time reaches 1/2 through a slot (t=SLOT_DURATION/2) THEN the Staker Node SHALL compute and store the safe target
2. WHEN the Staker Node computes the safe target THEN the Staker Node SHALL use the fork choice algorithm with 2/3 validator threshold

### Requirement 12

**User Story:** As a P2P network simulator, I want to manage message delivery between staker nodes with configurable latency, so that I can test the protocol under various network conditions.

#### Acceptance Criteria

1. WHEN a Staker Node submits a message to the P2P Network THEN the P2P Network SHALL schedule delivery to all other staker nodes
2. WHEN the P2P Network schedules a message THEN the P2P Network SHALL compute delivery time using the configured latency function
3. WHEN the P2P Network advances time THEN the P2P Network SHALL deliver all messages scheduled for that time or earlier
4. WHEN the P2P Network delivers a message THEN the P2P Network SHALL invoke the receive method on the recipient staker node
5. WHEN a Staker Node submits a message THEN the P2P Network SHALL not deliver the message back to the sender

### Requirement 13

**User Story:** As a protocol tester, I want to initialize a network with genesis state and multiple validators, so that I can simulate consensus scenarios.

#### Acceptance Criteria

1. WHEN the P2P Network initializes THEN the P2P Network SHALL start with time set to zero
2. WHEN a Staker Node initializes THEN the Staker Node SHALL accept a genesis block and genesis state
3. WHEN a Staker Node initializes THEN the Staker Node SHALL register itself with the P2P Network
4. WHEN a Staker Node initializes THEN the Staker Node SHALL store the genesis block hash and post-state
5. WHEN a Staker Node initializes THEN the Staker Node SHALL set the head to the genesis block hash

### Requirement 14

**User Story:** As a protocol tester, I want to run time-based simulations where nodes tick through protocol phases, so that I can observe consensus behavior over time.

#### Acceptance Criteria

1. WHEN the P2P Network advances by one time unit THEN the P2P Network SHALL increment the time counter
2. WHEN the P2P Network advances time THEN the P2P Network SHALL deliver all pending messages scheduled for delivery
3. WHEN a Staker Node ticks THEN the Staker Node SHALL execute the appropriate action based on the current time within the slot
4. WHEN the simulation runs THEN the Staker Node SHALL propose blocks at t=0, vote at t=SLOT_DURATION/4, compute safe target at t=SLOT_DURATION/2, and accept votes at t=3*SLOT_DURATION/4
