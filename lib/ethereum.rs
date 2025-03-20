use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Escrow,
    "abis/escrow.json"
);
