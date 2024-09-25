use async_trait::async_trait;
use mantis_common::UserIntent;
use solana_sdk::account_info::AccountInfo;
use solana_sdk::instruction::Instruction;

pub mod cmd;
mod ethereum;
mod solana;

#[allow(unused)]
enum Network {
    Solana(solana::Cluster),
    Ethereum(ethereum::Network),
}

pub trait EscrowContract {
    type Error;
    type Input<'a>;

    fn instruction(&self) -> Result<(Instruction, Vec<AccountInfo>), Self::Error>;

    fn submit_intent(&self, params: UserIntent) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Chain {
    type Transaction;
    type Address;
    type Token;
    type Amount;
    type Error;

    async fn get_transaction(&self, tx_hash: &str) -> Result<Self::Transaction, Self::Error>;

    // async fn process_transaction(
    //     &self,
    //     params: TransactionParams,
    // ) -> Result<bool, Self::Error>;
    //
    // async fn transfer(
    //     &self,
    //     recipient: &Self::Address,
    //     token: &Self::Token,
    //     amount: Self::Amount,
    // ) -> Result<String, Self::Error>;
    //
    // async fn create_token_account(
    //     &self,
    //     owner: &Self::Address,
    //     token: &Self::Token,
    // ) -> Result<(), Self::Error>;
    //
    // async fn get_current_block_number(&self) -> Result<u64, Self::Error>;

    async fn submit_intent(
        &self,
        intent: UserIntent,
        address: Self::Address,
    ) -> Result<(), Self::Error>;

    // async fn on_receive_transfer(
    //     &self,
    //     params: ReceiveTransferParams,
    //     program_id: Self::Address,
    // ) -> Result<(), Self::Error>;

    fn signer(&self) -> Self::Address;
}
