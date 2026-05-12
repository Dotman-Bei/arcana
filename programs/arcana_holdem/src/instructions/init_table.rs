use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::{HoldemError, Table, TableState, TABLE_SEED, TABLE_SPACE};

#[derive(Accounts)]
pub struct InitTable<'info> {
    #[account(mut)]
    pub player_a: Signer<'info>,
    #[account(
        init,
        payer = player_a,
        space = TABLE_SPACE,
        seeds = [TABLE_SEED, player_a.key().as_ref()],
        bump,
    )]
    pub table: Account<'info, Table>,
    pub system_program: Program<'info, System>,
}

pub fn init_table(ctx: Context<InitTable>, buy_in: u64, big_blind: u64, salt_a: u64, mask_commit: [u8; 32]) -> Result<()> {
    require!(big_blind > 0, HoldemError::InvalidBlind);
    require!(buy_in >= big_blind * 2, HoldemError::InvalidBuyIn);

    let table = &mut ctx.accounts.table;
    let player_a = ctx.accounts.player_a.key();

    // Transfer buy-in from player_a to table PDA.
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.player_a.to_account_info(),
                to: table.to_account_info(),
            },
        ),
        buy_in,
    )?;

    table.player_a = player_a;
    table.player_b = Pubkey::default();
    table.player_a_stack = buy_in;
    table.player_b_stack = 0;
    table.player_a_bet = 0;
    table.player_b_bet = 0;
    table.pot = 0;
    table.salt_a = salt_a;
    table.salt_b = 0;
    table.buy_in = buy_in;
    table.big_blind = big_blind;
    table.player_a_card1 = 0;
    table.player_a_card2 = 0;
    table.player_b_card1 = 0;
    table.player_b_card2 = 0;
    table.community = [0u8; 5];
    table.community_flags = 0;
    table.player_a_mask_commit = mask_commit;
    table.player_b_mask_commit = [0u8; 32]; // set when player B joins
    table.state = TableState::WaitingForPlayer;
    table.current_turn = 0;
    table.arcium_task_id = 0;
    table.state_nonce = 0;
    table.bump = ctx.bumps.table;

    Ok(())
}
