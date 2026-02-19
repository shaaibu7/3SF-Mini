# 3SF-Mini (Three Slot Finality - Minimal)

A Rust implementation of the 3SF-mini consensus protocol, which achieves finality within three slots under normal network conditions.

## Overview

3SF-mini is a simplified consensus protocol designed to demonstrate fast finality in blockchain systems. This implementation is based on the [Python reference implementation](https://github.com/ethereum/research/tree/master/3sf-mini) from the Ethereum research repository.

The protocol uses:
- **LMD GHOST** (Latest Message Driven Greedy Heaviest Observed SubTree) for fork choice
- **Justification and Finalization** rules similar to Casper FFG
- **View merge mechanism** for safety under network latency
- **Backoff justification slots** (≤5, perfect squares, or oblong numbers) to ensure progress under high latency

## Features

- ✅ Complete consensus logic with justification and finalization
- ✅ LMD GHOST fork choice algorithm
- ✅ P2P network simulator with configurable latency
- ✅ Multiple validator support
- ✅ Dependency resolution for out-of-order message delivery
- ✅ View merge mechanism for safety

## Architecture

The implementation consists of three main modules:

### Types Module (`src/types.rs`)
Defines core data structures:
- `Config`: Network configuration (number of validators)
- `State`: Consensus state tracking justified and finalized blocks
- `Vote`: Validator attestations with head, target, and source
- `Block`: Blockchain blocks with slots, parents, votes, and state roots

### Consensus Module (`src/consensus.rs`)
Implements consensus logic:
- `compute_hash()`: Deterministic hashing of consensus objects
- `is_justifiable_slot()`: Determines eligible slots for justification
- `process_block()`: Core state transition function
- `get_latest_justified_hash()`: Finds highest justified block
- `get_fork_choice_head()`: LMD GHOST fork choice algorithm

### P2P Module (`src/p2p.rs`)
Implements network simulation and validator nodes:
- `P2PNetwork`: Network simulator with configurable latency
- `Staker`: Validator node that proposes blocks and votes
- `Item`: Message type (Block or Vote)

## Building

```bash
cargo build --release
```

## Running the Simulation

```bash
cargo run
```

This runs a simulation with 4 validators for 200 time units, demonstrating:
- Block proposals every slot (12 time units)
- Voting at 1/4 through each slot
- Justification when 2/3 of validators agree
- Finalization when justified blocks have no intermediate justifiable slots

### Example Output

```
=== 3SF-Mini Simulation ===

Created 4 validators
Genesis block: 0000000000000000000000000000000000000000000000000000000000000000

--- Time: 0, Slot: 0 ---
  Latest Justified: 0000000000000000...
  Latest Finalized: 0000000000000000...
  Chain size: 1 blocks

--- Time: 48, Slot: 4 ---
  Latest Justified: f4bccf83e5fa9f86...
  Latest Finalized: 2318f6db49d54ce6...
  Chain size: 5 blocks
```

## Protocol Details

### Slot Timing

Each slot has a duration of 12 time units with specific protocol phases:
- **t=0**: Block proposal (if validator's turn)
- **t=3** (1/4 slot): Voting
- **t=6** (1/2 slot): Safe target computation
- **t=9** (3/4 slot): Vote acceptance deadline

### Justification

A block becomes justified when:
1. 2/3 of validators vote for it as their target
2. All votes source from an already-justified block
3. The target slot is justifiable (≤5 slots, perfect square, or oblong number from last finalized)

### Finalization

When a block B is justified, its source block S becomes finalized if there are no intermediate justifiable slots between S and B.

### Fork Choice (LMD GHOST)

The fork choice algorithm:
1. Starts from the latest justified block
2. Counts votes for each block (including descendant votes)
3. Repeatedly selects the child with the most votes
4. Tiebreaks by slot number, then hash lexicographically

## Testing

The implementation includes comprehensive unit tests and property-based tests (optional):

```bash
cargo test
```

## References

- [3SF-mini Python Reference Implementation](https://github.com/ethereum/research/tree/master/3sf-mini)
- [LMD GHOST Paper](https://arxiv.org/abs/2003.03052)
- [Casper FFG Paper](https://arxiv.org/abs/1710.09437)

## License

See LICENSE file for details.
