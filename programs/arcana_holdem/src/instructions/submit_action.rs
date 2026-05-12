use anchor_lang::prelude::*;

use crate::{GameSettled, HoldemError, PlayerAction, StreetAdvanced, Table, TableState, TABLE_SEED};

#[derive(Accounts)]
pub struct SubmitAction<'info> {
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [TABLE_SEED, table.player_a.as_ref()],
        bump = table.bump,
    )]
    pub table: Account<'info, Table>,
    /// CHECK: receives pot on a fold; ignored for all other actions.
    #[account(mut)]
    pub winner_wallet: UncheckedAccount<'info>,
}

pub fn submit_action(
    ctx: Context<SubmitAction>,
    action: PlayerAction,
    raise_to: u64,
) -> Result<()> {
    let player_key = ctx.accounts.player.key();

    let is_player_a = player_key == ctx.accounts.table.player_a;
    let is_player_b = player_key == ctx.accounts.table.player_b;
    require!(is_player_a || is_player_b, HoldemError::NotYourTurn);

    let acting = if is_player_a { 0u8 } else { 1u8 };
    require!(acting == ctx.accounts.table.current_turn, HoldemError::NotYourTurn);

    require!(
        ctx.accounts.table.state == TableState::PreFlop
            || ctx.accounts.table.state == TableState::Flop
            || ctx.accounts.table.state == TableState::Turn
            || ctx.accounts.table.state == TableState::River,
        HoldemError::InvalidState
    );

    match action {
        PlayerAction::Fold => {
            fold(&mut ctx.accounts.table, acting);
            let winner = 1 - acting;
            let winner_key = if winner == 0 {
                ctx.accounts.table.player_a
            } else {
                ctx.accounts.table.player_b
            };
            require!(ctx.accounts.winner_wallet.key() == winner_key, HoldemError::NotYourTurn);
            let pot = ctx.accounts.table.pot;
            ctx.accounts.table.pot = 0;
            ctx.accounts.table.state = TableState::Settled;
            **ctx.accounts.table.to_account_info().try_borrow_mut_lamports()? -= pot;
            **ctx.accounts.winner_wallet.try_borrow_mut_lamports()? += pot;
            emit!(GameSettled { table: ctx.accounts.table.key(), winner, pot });
        }

        PlayerAction::Check => {
            let my_bet  = if acting == 0 { ctx.accounts.table.player_a_bet } else { ctx.accounts.table.player_b_bet };
            let opp_bet = if acting == 0 { ctx.accounts.table.player_b_bet } else { ctx.accounts.table.player_a_bet };
            require!(my_bet >= opp_bet, HoldemError::CannotCheck);
            let table_key = ctx.accounts.table.key();
            let new_state_u8 = advance_street(&mut ctx.accounts.table, acting);
            emit!(StreetAdvanced { table: table_key, state: new_state_u8 });
        }

        PlayerAction::Call => {
            let my_bet  = if acting == 0 { ctx.accounts.table.player_a_bet } else { ctx.accounts.table.player_b_bet };
            let opp_bet = if acting == 0 { ctx.accounts.table.player_b_bet } else { ctx.accounts.table.player_a_bet };
            let my_stack = if acting == 0 { ctx.accounts.table.player_a_stack } else { ctx.accounts.table.player_b_stack };
            let to_call = opp_bet.saturating_sub(my_bet);
            require!(my_stack >= to_call, HoldemError::InsufficientStack);
            if acting == 0 {
                ctx.accounts.table.player_a_bet   += to_call;
                ctx.accounts.table.player_a_stack -= to_call;
            } else {
                ctx.accounts.table.player_b_bet   += to_call;
                ctx.accounts.table.player_b_stack -= to_call;
            }
            ctx.accounts.table.pot += to_call;
            let table_key = ctx.accounts.table.key();
            let new_state_u8 = advance_street(&mut ctx.accounts.table, acting);
            emit!(StreetAdvanced { table: table_key, state: new_state_u8 });
        }

        PlayerAction::Raise => {
            let opp_bet  = if acting == 0 { ctx.accounts.table.player_b_bet } else { ctx.accounts.table.player_a_bet };
            let my_bet   = if acting == 0 { ctx.accounts.table.player_a_bet } else { ctx.accounts.table.player_b_bet };
            let my_stack = if acting == 0 { ctx.accounts.table.player_a_stack } else { ctx.accounts.table.player_b_stack };
            let min_raise = opp_bet + ctx.accounts.table.big_blind;
            require!(raise_to >= min_raise, HoldemError::InvalidRaise);
            let additional = raise_to.saturating_sub(my_bet);
            require!(my_stack >= additional, HoldemError::InsufficientStack);
            if acting == 0 {
                ctx.accounts.table.player_a_bet    = raise_to;
                ctx.accounts.table.player_a_stack -= additional;
            } else {
                ctx.accounts.table.player_b_bet    = raise_to;
                ctx.accounts.table.player_b_stack -= additional;
            }
            ctx.accounts.table.pot += additional;
            ctx.accounts.table.current_turn = 1 - acting;
        }
    }

    Ok(())
}

fn fold(table: &mut Table, acting: u8) {
    // No state change needed here — caller handles pot transfer and Settled state.
    let _ = acting;
    let _ = table;
}

/// Advance to the next street. Resets per-street bets and updates community_flags.
/// Returns a u8 identifying the new state for the StreetAdvanced event.
fn advance_street(table: &mut Table, acting: u8) -> u8 {
    table.player_a_bet = 0;
    table.player_b_bet = 0;

    let (new_state, state_u8) = match table.state {
        TableState::PreFlop => {
            table.community_flags |= 0b00000111;
            (TableState::Flop, 3u8)
        }
        TableState::Flop => {
            table.community_flags |= 0b00001000;
            (TableState::Turn, 4u8)
        }
        TableState::Turn => {
            table.community_flags |= 0b00010000;
            (TableState::River, 5u8)
        }
        TableState::River => (TableState::Showdown, 6u8),
        _ => return 0,
    };

    table.state = new_state;
    // Post-flop: opponent of the last actor goes first.
    table.current_turn = 1 - acting;
    state_u8
}
