use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CircuitSource;

pub mod errors;
pub mod instructions;

pub use errors::*;
pub use instructions::{
    CloseTable, DealCallbackOutput, InitTable,
    RevealShowdown, SubmitAction,
};
pub(crate) use instructions::{
    __client_accounts_close_table,
    __client_accounts_init_table,
    __client_accounts_reveal_showdown,
    __client_accounts_submit_action,
};

// Placeholder — replaced with the actual program ID after `anchor build && anchor deploy`.
declare_id!("ETDyB55bRcVFF8sJrjc5oLwJ6KevLchoo3etzvkG7RHC");

/// Byte offset of the encrypted_state field inside the serialised Table account.
///
/// Layout (Borsh, 8-byte discriminator then fields in declaration order):
///   8   discriminator
///  32   player_a
///  32   player_b
///   8   player_a_stack
///   8   player_b_stack
///   8   player_a_bet
///   8   player_b_bet
///   8   pot
///   8   salt_a
///   8   salt_b
///   8   buy_in
///   8   big_blind
///   8   player_a_card1   (masked hole card)
///   8   player_a_card2
///   8   player_b_card1
///   8   player_b_card2
///   5   community        ([u8; 5])
///   1   community_flags  (bitmask: bits 0-2=flop, 3=turn, 4=river)
///  32   player_a_mask_commit
///  32   player_b_mask_commit
///   1   state            (enum variant as u8 in Borsh)
///   1   current_turn
///   8   arcium_task_id
///  16   state_nonce      (u128 for MXE ciphertext nonce)
///   1   bump
/// ---
/// 232 bytes before any encrypted_state field (not used here — we use ArgBuilder)
pub const ENCRYPTED_STATE_OFFSET: u32 = 232;

/// comp_def_offset for the deal_cards computation definition.
/// Must match the function name in arcium_compute/src/lib.rs.
pub const COMP_DEF_OFFSET_DEAL_CARDS: u32 = comp_def_offset("deal_cards");

/// Space allocation for the Table account.
/// 232 bytes of fixed fields + 8 discriminator + padding = 512
pub const TABLE_SPACE: usize = 512;

/// TABLE_SEED prefix used in PDA derivation: ["holdem", player_a_key]
pub const TABLE_SEED: &[u8] = b"holdem";

#[arcium_program]
pub mod arcana_holdem {
    use super::*;
    use crate::instructions::{DealCardsCallback, JoinTable};
    use crate::instructions::{__client_accounts_deal_cards_callback, __client_accounts_join_table};

    // ── One-time setup ────────────────────────────────────────────────────────

    /// Register the `deal_cards` computation definition with the MXE.
    /// `arcis_url` is the public URL of the compiled deal_cards.arcis circuit file
    /// (uploaded to Supabase or another public host after `anchor build`).
    pub fn init_deal_cards_comp_def(ctx: Context<InitDealCardsCompDef>, arcis_url: String) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::Url(arcis_url)), None)?;
        Ok(())
    }

    // ── Game lifecycle ────────────────────────────────────────────────────────

    /// Player A creates the table, sets buy-in and big blind, deposits SOL.
    /// `mask_commit` = SHA-256(p1_mask_le || table_pubkey) — committed before the deal.
    pub fn init_table(
        ctx: Context<InitTable>,
        buy_in: u64,
        big_blind: u64,
        salt_a: u64,
        mask_commit: [u8; 32],
    ) -> Result<()> {
        instructions::init_table(ctx, buy_in, big_blind, salt_a, mask_commit)
    }

    /// Player B joins the table, deposits SOL, and triggers the deal via Arcium MXE.
    /// `mask_commit_b` = SHA-256(p2_mask_le || table_pubkey).
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
        instructions::join_table(
            ctx,
            salt_b,
            mask_commit_b,
            computation_offset,
            deck_input_pubkey,
            deck_input_nonce,
            seed_encrypted,
            p1_mask_encrypted,
            p2_mask_encrypted,
        )
    }

    /// Arcium callback — invoked by the MXE after deal_cards completes.
    /// Stores masked hole cards and community cards on the Table PDA.
    #[arcium_callback(encrypted_ix = "deal_cards")]
    pub fn deal_cards_callback(
        ctx: Context<DealCardsCallback>,
        output: SignedComputationOutputs<DealCallbackOutput>,
    ) -> Result<()> {
        instructions::deal_cards_callback(ctx, output)
    }

    /// A player acts: Fold, Check, Call, or Raise.
    /// Advances the betting round or game phase as appropriate.
    pub fn submit_action(
        ctx: Context<SubmitAction>,
        action: PlayerAction,
        raise_to: u64,
    ) -> Result<()> {
        instructions::submit_action(ctx, action, raise_to)
    }

    /// At showdown, each player calls this once to reveal their XOR mask.
    /// Once both masks are revealed the contract evaluates hands and settles the pot.
    pub fn reveal_showdown(ctx: Context<RevealShowdown>, mask: u64) -> Result<()> {
        instructions::reveal_showdown(ctx, mask)
    }

    /// Reclaim rent after the game is Settled.
    pub fn close_table(ctx: Context<CloseTable>) -> Result<()> {
        instructions::close_table(ctx)
    }
}

// ── On-chain state ────────────────────────────────────────────────────────────

#[account]
pub struct Table {
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub player_a_stack: u64,
    pub player_b_stack: u64,
    /// Outstanding bet by each player in the current street.
    pub player_a_bet: u64,
    pub player_b_bet: u64,
    pub pot: u64,
    /// Random salts contributed by each player for deck seed derivation.
    pub salt_a: u64,
    pub salt_b: u64,
    pub buy_in: u64,
    pub big_blind: u64,
    /// Hole cards masked: card_raw XOR player_mask. Player decrypts client-side.
    pub player_a_card1: u64,
    pub player_a_card2: u64,
    pub player_b_card1: u64,
    pub player_b_card2: u64,
    /// Community cards (0-51). Index 0-2 = flop, 3 = turn, 4 = river.
    /// Stored after deal; access gated by community_flags in the frontend.
    pub community: [u8; 5],
    /// Bitmask controlling which community cards are "live" for the current street.
    /// bit 0..2 = flop cards active, bit 3 = turn active, bit 4 = river active.
    pub community_flags: u8,
    /// Keccak256 commitments to each player's mask: hash(mask || table_pubkey).
    /// Set when the player calls join_table (A) or init_table (B) with their mask.
    pub player_a_mask_commit: [u8; 32],
    pub player_b_mask_commit: [u8; 32],
    pub state: TableState,
    /// 0 = player_a's turn, 1 = player_b's turn.
    pub current_turn: u8,
    pub arcium_task_id: u64,
    pub state_nonce: u128,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum TableState {
    WaitingForPlayer,
    Dealing,
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    /// Set to Settled once reveal_showdown determines the winner.
    Settled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Raise,
}

#[event]
pub struct CardsDealt {
    pub table: Pubkey,
}

#[event]
pub struct StreetAdvanced {
    pub table: Pubkey,
    pub state: u8,
}

#[event]
pub struct GameSettled {
    pub table: Pubkey,
    /// 0 = player_a wins, 1 = player_b wins, 2 = split pot.
    pub winner: u8,
    pub pot: u64,
}

// ── One-time setup context ────────────────────────────────────────────────────

#[init_computation_definition_accounts("deal_cards", payer)]
#[derive(Accounts)]
pub struct InitDealCardsCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account verified by the Arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table verified by the Arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

// ArciumSignerAccount is generated automatically by the #[arcium_program] macro.
