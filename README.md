# Mantis CLI 🦀

A CLI utility to interact with the Mantis smart contracts.

```sh
cargo install --path=mantis-cli
```

To perform a swap from USDC to USDT on Solana with 5 min timeout:

```sh
mantis-cli swap --src-chain=solana --dst-chain=solana --token-in=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount-in=10000000 --token-out=Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB  --amount-out=1 --timeout=300
```

To perform a cross-chain swap from ETH on Ethereum to SOL on Solana with 10 min timeout:

```sh
mantis-cli swap --src-chain=ethereum --dst-chain=solana --token-in=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE --amount-in=10000000000000000 --token-out=11111111111111111111111111111111 --amount-out=1 --timeout=600
```

To cancel an existing intent on Solana:

```sh
mantis-cli cancel --src-chain=solana --intent-id=123456789000
```

# Mantis SDK 🦀

A Rust SDK library to interact with the Mantis smart contracts.

The `mantis_sdk::ethereum` module abstracts the Ethereum smart contract interactions and provides various utility functions.

The `mantis_sdk::solana` module abstracts the Solana Anchor program interactions and provides various utility functions.

The `mantis_sdk::auction` module provides a way for solvers to integrate with the intent auction process by communicating with the auctioneer API via a robust WebSocket client:

# Mantis SDK 🔷

A TypeScript SDK library to interact with the Mantis smart contracts.

The `ethereum` module abstracts the Ethereum smart contract interactions and provides various utility functions.

The `solana` module abstracts the Solana Anchor program interactions and provides various utility functions.
