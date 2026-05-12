use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;

use crate::{GameSettled, HoldemError, Table, TableState, TABLE_SEED};

#[derive(Accounts)]
pub struct RevealShowdown<'info> {
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [TABLE_SEED, table.player_a.as_ref()],
        bump = table.bump,
        constraint = table.state == TableState::Showdown @ HoldemError::NotShowdown,
    )]
    pub table: Account<'info, Table>,
    /// CHECK: receives pot on Player A win or split.
    #[account(mut)]
    pub player_a_wallet: UncheckedAccount<'info>,
    /// CHECK: receives pot on Player B win or split.
    #[account(mut)]
    pub player_b_wallet: UncheckedAccount<'info>,
}

/// Each player calls this once to reveal their XOR mask.
/// On the second reveal the contract evaluates hands and settles the pot.
pub fn reveal_showdown(ctx: Context<RevealShowdown>, mask: u64) -> Result<()> {
    let player_key  = ctx.accounts.player.key();
    let table_key   = ctx.accounts.table.key();
    let player_a_key = ctx.accounts.table.player_a;
    let player_b_key = ctx.accounts.table.player_b;

    let is_a = player_key == player_a_key;
    let is_b = player_key == player_b_key;
    require!(is_a || is_b, HoldemError::NotYourTurn);

    // Verify mask against committed hash: SHA-256(mask_le || table_pubkey)
    let mut preimage = [0u8; 40];
    preimage[..8].copy_from_slice(&mask.to_le_bytes());
    preimage[8..].copy_from_slice(&table_key.to_bytes());
    let hash = keccak::hash(&preimage);

    if is_a {
        require!(hash.0 == ctx.accounts.table.player_a_mask_commit, HoldemError::InvalidMaskReveal);
        ctx.accounts.table.player_a_card1 ^= mask;
        ctx.accounts.table.player_a_card2 ^= mask;
        ctx.accounts.table.player_a_mask_commit = [0u8; 32]; // signal: A has revealed
    } else {
        require!(hash.0 == ctx.accounts.table.player_b_mask_commit, HoldemError::InvalidMaskReveal);
        ctx.accounts.table.player_b_card1 ^= mask;
        ctx.accounts.table.player_b_card2 ^= mask;
        ctx.accounts.table.player_b_mask_commit = [0u8; 32]; // signal: B has revealed
    }

    // Wait for both players to reveal (both commits zeroed).
    if ctx.accounts.table.player_a_mask_commit != [0u8; 32]
        || ctx.accounts.table.player_b_mask_commit != [0u8; 32]
    {
        return Ok(());
    }

    // ── Both masks revealed — evaluate and settle ─────────────────────────────
    // Collect everything we need before dropping the table borrow.
    let (winner, pot) = {
        let t = &mut ctx.accounts.table;
        let p1_hand = [
            t.player_a_card1 as u8, t.player_a_card2 as u8,
            t.community[0], t.community[1], t.community[2], t.community[3], t.community[4],
        ];
        let p2_hand = [
            t.player_b_card1 as u8, t.player_b_card2 as u8,
            t.community[0], t.community[1], t.community[2], t.community[3], t.community[4],
        ];
        let p1_rank = best_hand_of_seven(p1_hand);
        let p2_rank = best_hand_of_seven(p2_hand);

        let winner: u8 = if p1_rank > p2_rank { 0 } else if p2_rank > p1_rank { 1 } else { 2 };
        let pot = t.pot;
        t.pot = 0;
        t.state = TableState::Settled;
        (winner, pot)
    };
    // Mutable borrow of table is released here — safe to do lamport transfers.

    require!(ctx.accounts.player_a_wallet.key() == player_a_key, HoldemError::NotYourTurn);
    require!(ctx.accounts.player_b_wallet.key() == player_b_key, HoldemError::NotYourTurn);

    match winner {
        0 => {
            **ctx.accounts.table.to_account_info().try_borrow_mut_lamports()? -= pot;
            **ctx.accounts.player_a_wallet.try_borrow_mut_lamports()? += pot;
        }
        1 => {
            **ctx.accounts.table.to_account_info().try_borrow_mut_lamports()? -= pot;
            **ctx.accounts.player_b_wallet.try_borrow_mut_lamports()? += pot;
        }
        _ => {
            // Split pot; odd lamport goes to player_a.
            let half = pot / 2;
            let remainder = pot - half * 2;
            **ctx.accounts.table.to_account_info().try_borrow_mut_lamports()? -= pot;
            **ctx.accounts.player_a_wallet.try_borrow_mut_lamports()? += half + remainder;
            **ctx.accounts.player_b_wallet.try_borrow_mut_lamports()? += half;
        }
    }

    emit!(GameSettled { table: table_key, winner, pot });
    Ok(())
}

// ── Hand evaluation ───────────────────────────────────────────────────────────

fn best_hand_of_seven(cards: [u8; 7]) -> u64 {
    const COMBOS: [[usize; 5]; 21] = [
        [0,1,2,3,4],[0,1,2,3,5],[0,1,2,3,6],
        [0,1,2,4,5],[0,1,2,4,6],[0,1,2,5,6],
        [0,1,3,4,5],[0,1,3,4,6],[0,1,3,5,6],
        [0,1,4,5,6],[0,2,3,4,5],[0,2,3,4,6],
        [0,2,3,5,6],[0,2,4,5,6],[0,3,4,5,6],
        [1,2,3,4,5],[1,2,3,4,6],[1,2,3,5,6],
        [1,2,4,5,6],[1,3,4,5,6],[2,3,4,5,6],
    ];
    COMBOS.iter().map(|c| eval_five([cards[c[0]], cards[c[1]], cards[c[2]], cards[c[3]], cards[c[4]]])).max().unwrap_or(0)
}

fn eval_five(hand: [u8; 5]) -> u64 {
    let ranks: [u64; 5] = hand.map(|c| (c % 13) as u64);
    let suits: [u64; 5] = hand.map(|c| (c / 13) as u64);

    let is_flush = suits[0] == suits[1] && suits[1] == suits[2] && suits[2] == suits[3] && suits[3] == suits[4];

    let mut sorted = ranks;
    sorted.sort_unstable();

    let is_straight_high = sorted[4] == sorted[0] + 4
        && sorted[1] == sorted[0] + 1
        && sorted[2] == sorted[0] + 2
        && sorted[3] == sorted[0] + 3;
    let is_straight_low = sorted == [0, 1, 2, 3, 12]; // A-2-3-4-5
    let is_straight = is_straight_high || is_straight_low;

    let mut freq = [0u64; 13];
    for &r in &ranks { freq[r as usize] += 1; }

    let (mut pairs, mut trips, mut quads) = (0u64, 0u64, 0u64);
    let (mut pair_rank, mut trip_rank, mut quad_rank) = (0u64, 0u64, 0u64);
    for (i, &f) in freq.iter().enumerate() {
        match f {
            2 => { pairs += 1; pair_rank = i as u64; }
            3 => { trips = 1;  trip_rank = i as u64; }
            4 => { quads = 1;  quad_rank = i as u64; }
            _ => {}
        }
    }

    let tb = sorted[4] * 371_293 + sorted[3] * 28_561 + sorted[2] * 2_197 + sorted[1] * 169 + sorted[0] * 13;

    if is_flush && is_straight { return 8 * 10_000_000_000 + tb; }
    if quads == 1              { return 7 * 10_000_000_000 + quad_rank * 100_000_000; }
    if trips == 1 && pairs == 1{ return 6 * 10_000_000_000 + trip_rank * 100_000_000 + pair_rank; }
    if is_flush                { return 5 * 10_000_000_000 + tb; }
    if is_straight             { return 4 * 10_000_000_000 + tb; }
    if trips == 1              { return 3 * 10_000_000_000 + trip_rank * 100_000_000; }
    if pairs == 2              { return 2 * 10_000_000_000 + tb; }
    if pairs == 1              { return 1 * 10_000_000_000 + pair_rank * 100_000_000 + tb; }
    tb
}
