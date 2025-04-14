# Mantis CLI 🦀

A CLI utility to interact with the Mantis smart contracts.

```sh
cargo install --path=mantis-cli
```

To perform a swap from USDC to USDT on Solana with 5 min timeout:

```sh
mantis-cli swap --src_chain=solana --dst_chain=solana --dst_user=4SsL3qQCbE4ff2PnZmAExLye85GETnsVZgouapAL7fGn --token_in=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --token_out=Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB --amount_in=10000000 --amount_out=1 --timeout=300
```

To perform a cross-chain swap from ETH on Ethereum to SOL on Solana with 10 min timeout:

```sh
mantis-cli swap --src_chain=ethereum --dst_chain=solana --dst_user=4SsL3qQCbE4ff2PnZmAExLye85GETnsVZgouapAL7fGn --token_in=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE --token_out=11111111111111111111111111111111 --amount_in=10000000000000000 --amount_out=1 --timeout=600
```

To cancel an existing intent on Solana:

```sh
mantis-cli cancel --src_chain=solana --intent_id=123456789000
```

# Mantis SDK 🦀

A Rust SDK library to interact with the Mantis smart contracts.

The `mantis_sdk::ethereum` module abstracts the Ethereum smart contract interactions and provides various utility functions.

The `mantis_sdk::solana` module abstracts the Solana Anchor program interactions and provides various utility functions.

The `mantis_sdk::auction` module provides a way for solvers to integrate with the intent auction process by communicating with the auctioneer API.

# Mantis SDK 🔷

A TypeScript SDK library to interact with the Mantis smart contracts.

The `ethereum` module abstracts the Ethereum smart contract interactions and provides various utility functions.

The `solana` module abstracts the Solana Anchor program interactions and provides various utility functions.
