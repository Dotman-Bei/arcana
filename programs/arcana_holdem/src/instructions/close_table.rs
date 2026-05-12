use anchor_lang::prelude::*;

use crate::{HoldemError, Table, TableState, TABLE_SEED};

#[derive(Accounts)]
pub struct CloseTable<'info> {
    #[account(mut)]
    pub player_a: Signer<'info>,
    #[account(
        mut,
        close = player_a,
        seeds = [TABLE_SEED, player_a.key().as_ref()],
        bump = table.bump,
        constraint = table.state == TableState::Settled @ HoldemError::InvalidState,
        constraint = table.player_a == player_a.key() @ HoldemError::NotYourTurn,
    )]
    pub table: Account<'info, Table>,
    pub system_program: Program<'info, System>,
}

pub fn close_table(_ctx: Context<CloseTable>) -> Result<()> {
    Ok(())
}
