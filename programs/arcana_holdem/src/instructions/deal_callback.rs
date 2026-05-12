use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

use crate::{CardsDealt, HoldemError, TableState, COMP_DEF_OFFSET_DEAL_CARDS};

/// Output from the deal_cards garbled circuit: [u64; 9]
///   [0..1] = player_a masked hole cards
///   [2..3] = player_b masked hole cards
///   [4..8] = community cards (0-51 cast to u64)
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DealCallbackOutput {
    pub field_0: [u64; 9],
}

impl arcium_anchor::HasSize for DealCallbackOutput {
    const SIZE: usize = 9 * 8; // 9 × u64 = 72 bytes
}

/// Arcium requires the callback accounts struct to be named
/// `<PascalCase(instruction_name)>Callback` → `DealCardsCallback`.
#[callback_accounts("deal_cards")]
#[derive(Accounts)]
pub struct DealCardsCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEAL_CARDS))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account verified by the Arcium program.
    pub computation_account: UncheckedAccount<'info>,
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar.
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub table: Account<'info, Table>,
}

pub fn deal_callback(
    ctx: Context<DealCardsCallback>,
    output: SignedComputationOutputs<DealCallbackOutput>,
) -> Result<()> {
    let cards = match output.verify_output(
        &ctx.accounts.cluster_account,
        &ctx.accounts.computation_account,
    ) {
        Ok(DealCallbackOutput { field_0 }) => field_0,
        Err(_) => return err!(HoldemError::InvalidState),
    };

    let table_key = ctx.accounts.table.key();
    let table = &mut ctx.accounts.table;

    require!(table.state == TableState::Dealing, HoldemError::InvalidState);

    table.player_a_card1 = cards[0];
    table.player_a_card2 = cards[1];
    table.player_b_card1 = cards[2];
    table.player_b_card2 = cards[3];

    table.community[0] = cards[4] as u8;
    table.community[1] = cards[5] as u8;
    table.community[2] = cards[6] as u8;
    table.community[3] = cards[7] as u8;
    table.community[4] = cards[8] as u8;

    table.state = TableState::PreFlop;

    emit!(CardsDealt { table: table_key });

    Ok(())
}
