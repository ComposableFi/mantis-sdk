use anchor_lang::prelude::*;
use mantis_common::UserIntent;

declare_id!("8DFWLK3ADs4qm1s394g9H7Vfyngad7DBpNXiaERoAvED");

#[program]
pub mod mantis_escrow_program {
    use super::*;

    pub fn escrow_and_store_intent(
        _ctx: Context<Initialize>,
        _amount: u64,
        _new_intent: UserIntent,
    ) -> Result<()> {
        unimplemented!()
    }
}

#[derive(Accounts)]
pub struct Initialize {}
