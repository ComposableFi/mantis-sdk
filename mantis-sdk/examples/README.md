# Solver Examples

This directory contains example solvers demonstrating how to use the Mantis SDK:

1. **minimal_solver.rs** - The simplest possible solver implementation
2. **simple_solver.rs** - A more complete example with proper structure and error handling

## Overview

This example shows:
- Connecting to the auctioneer via WebSocket
- Registering as a solver
- Receiving and bidding on auctions
- Executing swaps when winning auctions
- Handling quotes

## Quick Start

1. Copy the example environment file and fill in your values:

```bash
cd examples
cp .env.example .env
# Edit .env with your private keys and configuration
```

2. Run the solvers:

```bash
# Minimal example
cargo run --example minimal_solver

# Simple example with more features
cargo run --example simple_solver
```

## Environment Variables

The examples use the following environment variables (loaded from `.env` file):

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `SOLVER_ID` | Unique identifier for your solver (5-15 alphanumeric chars) | No | `simple001` or `minimal001` |
| `AUCTIONEER_WS_URL` | WebSocket URL of the auctioneer service | No | `ws://localhost:8080/auction` |
| `ETHEREUM_PRIVATE_KEY` | Ethereum private key (hex without 0x prefix) | Yes | - |
| `SOLANA_PRIVATE_KEY` | Solana private key (base58 encoded) | Yes | - |
| `COMMISSION_BPS` | Commission in basis points (100 = 1%) | No | `100` |

## Extending the Example(s)

To build a production solver:
1. Implement real swap execution via DEX integrations
2. Add sophisticated pricing and profitability models
3. Implement proper error recovery and retry logic
4. Add monitoring and metrics
5. Support more chains (Base, etc.)
6. Implement actual cross-chain bridging
