#![allow(unused)]
use anchor_client::{
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, signature::Keypair,
        signer::Signer, system_program,
    },
    Client, Cluster,
};
use anchor_lang::prelude::*;
use std::rc::Rc;

declare_program!(escrow);
use escrow::{accounts::Intent, client::accounts, client::args};
