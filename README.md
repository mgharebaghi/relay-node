# Centichain

Centichain is a decentralized blockchain network built with Rust and libp2p. It implements a relay-based networking architecture for secure and efficient peer-to-peer communication.

## Features

- Decentralized peer-to-peer networking using libp2p
- Relay-based architecture for improved connectivity
- Handshaking and connection management between nodes
- Transaction and block propagation
- Leader election and voting mechanisms
- Database integration for persistent storage
- Gossipsub for pub/sub messaging
- Request/response protocols for node communication

## Architecture

The system consists of relay nodes that facilitate connections between validators in the network. Key components include:

- Connection handling and peer management
- Block and transaction processing
- Leader election and consensus mechanisms
- Event-driven networking using libp2p SwarmEvents
- Database integration for storing blockchain data

## Getting Started

### Prerequisites

- Rust toolchain
- Cargo package manager

### Building
