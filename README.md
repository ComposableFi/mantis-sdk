# Mantis CLI 🦀

A CLI utility to interact with the Mantis smart contracts.

```sh
cargo install --path=mantis-cli
```

To perform a swap from USDC to USDT on Solana with 5 min timeout:

```sh
mantis-cli swap --src_chain=solana --dst_chain=solana --token_in=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --token_out=Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB --amount_in=10000000 --amount_out=1 --timeout=300
```

# Mantis SDK 🦀

A Rust SDK library to interact with the Mantis smart contracts.

# Auction SDK 🦀

A Rust SDK library for solvers to integrate with the auction process.

# Mantis SDK 🔷

A TypeScript SDK library to interact with the Mantis smart contracts.
