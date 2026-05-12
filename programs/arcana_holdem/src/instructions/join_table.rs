use anchor_lang::prelude::*;
use anchor_lang::system_program;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

use crate::{
    ArciumSignerAccount, DealCallback, HoldemError, Table, TableState,
    COMP_DEF_OFFSET_DEAL_CARDS, ID, ID_CONST, TABLE_SEED,
};

/// Player B joins the table and immediately triggers the Arcium MXE deal.
/// Blinds are posted atomically so the game moves straight to Dealing.
#[queue_computation_accounts("deal_cards", player_b)]
#[derive(Accounts)]
#[instruction(salt_b: u64, mask_commit_b: [u8; 32], computation_offset: u64)]
pub struct JoinTable<'info> {
    #[account(mut)]
    pub player_b: Signer<'info>,
    #[account(
        mut,
        seeds = [TABLE_SEED, table.player_a.as_ref()],
        bump = table.bump,
        constraint = table.state == TableState::WaitingForPlayer @ HoldemError::InvalidState,
        constraint = table.player_b == Pubkey::default() @ HoldemError::TableFull,
        constraint = player_b.key() != table.player_a @ HoldemError::AlreadySeated,
    )]
    pub table: Account<'info, Table>,
    #[account(
        init_if_needed,
        space = 9,
        payer = player_b,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, HoldemError::ClusterNotSet))]
    /// CHECK: verified by the Arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, HoldemError::ClusterNotSet))]
    /// CHECK: verified by the Arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, HoldemError::ClusterNotSet))]
    /// CHECK: verified by the Arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEAL_CARDS))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, HoldemError::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[allow(clippy::too_many_arguments)]
pub fn join_table(
    ctx: Context<JoinTable>,
    salt_b: u64,
    mask_commit_b: [u8; 32],
    computation_offset: u64,
    deck_input_pubkey: [u8; 32],
    deck_input_nonce: u128,
    seed_encrypted: [u8; 32],
    p1_mask_encrypted: [u8; 32],
    p2_mask_encrypted: [u8; 32],
) -> Result<()> {
    let buy_in    = ctx.accounts.table.buy_in;
    let big_blind = ctx.accounts.table.big_blind;
    let small_blind = big_blind / 2;

    // Transfer player_b's buy-in to the table PDA.
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.player_b.to_account_info(),
                to: ctx.accounts.table.to_account_info(),
            },
        ),
        buy_in,
    )?;

    ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

    let table_key = ctx.accounts.table.key();
    let callback_ixs = vec![DealCallback::callback_ix(
        computation_offset,
        &ctx.accounts.mxe_account,
        &[CallbackAccount { pubkey: table_key, is_writable: true }],
    )?];

    // DeckInput struct: { seed: u64, p1_mask: u64, p2_mask: u64 }
    // Three encrypted_u64 fields share one x25519 pubkey + nonce.
    let args = ArgBuilder::new()
        .x25519_pubkey(deck_input_pubkey)
        .plaintext_u128(deck_input_nonce)
        .encrypted_u64(seed_encrypted)
        .encrypted_u64(p1_mask_encrypted)
        .encrypted_u64(p2_mask_encrypted)
        .build();

    queue_computation(ctx.accounts, computation_offset, args, callback_ixs, 1, 0)?;

    // Mutate table only after queue_computation releases the borrow.
    let table = &mut ctx.accounts.table;
    table.player_b = ctx.accounts.player_b.key();
    table.player_b_stack = buy_in;
    table.salt_b = salt_b;
    table.player_b_mask_commit = mask_commit_b;

    // Post blinds: player_a = dealer/SB, player_b = BB.
    table.player_a_bet    = small_blind;
    table.player_b_bet    = big_blind;
    table.player_a_stack -= small_blind;
    table.player_b_stack -= big_blind;
    table.pot             = small_blind + big_blind;

    table.arcium_task_id = computation_offset;
    table.state          = TableState::Dealing;
    table.current_turn   = 0; // player_a acts first pre-flop (left of BB)

    Ok(())
}
