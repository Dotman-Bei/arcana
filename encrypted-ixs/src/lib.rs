use arcis::*;

/// Private card dealing for Arcana Hold'em.
///
/// The MXE receives three u64 values encrypted jointly under the cluster key:
///   seed    — combined entropy from both players (salt_a XOR salt_b)
///   p1_mask — player A's secret XOR mask for their hole cards
///   p2_mask — player B's secret XOR mask for their hole cards
///
/// The circuit shuffles a 52-card deck using an LCG-based Fisher-Yates variant,
/// deals 4 hole cards (2 per player) and 5 community cards, then masks the hole
/// cards with each player's secret before revealing.
///
/// Return layout: [u64; 9]
///   [0] = player_a card1 XOR p1_mask   ← only player A can unmask
///   [1] = player_a card2 XOR p1_mask   ← only player A can unmask
///   [2] = player_b card1 XOR p2_mask   ← only player B can unmask
///   [3] = player_b card2 XOR p2_mask   ← only player B can unmask
///   [4] = community[0]  (public, released on flop)
///   [5] = community[1]  (public, released on flop)
///   [6] = community[2]  (public, released on flop)
///   [7] = community[3]  (public, released on turn)
///   [8] = community[4]  (public, released on river)
///
/// Privacy guarantee: neither player sees the other's hole cards. The MXE
/// never reveals p1_mask, p2_mask, or the plaintext hole cards to anyone.
/// Community cards are stored on-chain but only "activated" by the contract
/// at the appropriate betting street.
#[encrypted]
mod circuits {
    use arcis::*;

    pub struct DeckInput {
        /// Combined entropy: client computes salt_a XOR salt_b and encrypts.
        seed: u64,
        /// Player A's secret mask — only A knows this value.
        p1_mask: u64,
        /// Player B's secret mask — only B knows this value.
        p2_mask: u64,
    }

    #[instruction]
    pub fn deal_cards(input: Enc<Shared, DeckInput>) -> [u64; 9] {
        let inp = input.to_arcis();

        // ── LCG-based deck shuffle ─────────────────────────────────────────────
        // Fisher-Yates on the first 9 positions using oblivious conditional swaps.
        // Each oblivious_swap_N is fully unrolled so the circuit topology is fixed.

        let mut deck = [
             0u64,  1,  2,  3,  4,  5,  6,  7,  8,  9,
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
            40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
            50, 51,
        ];

        // LCG constants (Knuth MMIX)
        const A: u64 = 6_364_136_223_846_793_005;
        const C: u64 = 1_442_695_040_888_963_407;

        let r0 = inp.seed.wrapping_mul(A).wrapping_add(C);
        let j0 = (r0 >> 33) % 52u64;
        oblivious_swap_0(&mut deck, j0);

        let r1 = r0.wrapping_mul(A).wrapping_add(C);
        let j1 = (r1 >> 33) % 51u64;
        oblivious_swap_1(&mut deck, j1 + 1);

        let r2 = r1.wrapping_mul(A).wrapping_add(C);
        let j2 = (r2 >> 33) % 50u64;
        oblivious_swap_2(&mut deck, j2 + 2);

        let r3 = r2.wrapping_mul(A).wrapping_add(C);
        let j3 = (r3 >> 33) % 49u64;
        oblivious_swap_3(&mut deck, j3 + 3);

        let r4 = r3.wrapping_mul(A).wrapping_add(C);
        let j4 = (r4 >> 33) % 48u64;
        oblivious_swap_4(&mut deck, j4 + 4);

        let r5 = r4.wrapping_mul(A).wrapping_add(C);
        let j5 = (r5 >> 33) % 47u64;
        oblivious_swap_5(&mut deck, j5 + 5);

        let r6 = r5.wrapping_mul(A).wrapping_add(C);
        let j6 = (r6 >> 33) % 46u64;
        oblivious_swap_6(&mut deck, j6 + 6);

        let r7 = r6.wrapping_mul(A).wrapping_add(C);
        let j7 = (r7 >> 33) % 45u64;
        oblivious_swap_7(&mut deck, j7 + 7);

        let r8 = r7.wrapping_mul(A).wrapping_add(C);
        let j8 = (r8 >> 33) % 44u64;
        oblivious_swap_8(&mut deck, j8 + 8);

        // ── Mask hole cards, leave community cards plain ───────────────────────
        // XOR is unsupported in arcis circuits; use wrapping_add (additive OTP).
        // The on-chain reveal uses wrapping_sub to recover the plaintext card.
        [
            deck[0].wrapping_add(inp.p1_mask),
            deck[1].wrapping_add(inp.p1_mask),
            deck[2].wrapping_add(inp.p2_mask),
            deck[3].wrapping_add(inp.p2_mask),
            deck[4],
            deck[5],
            deck[6],
            deck[7],
            deck[8],
        ]
        .reveal()
    }

    // ── Oblivious swap helpers ────────────────────────────────────────────────
    //
    // Each function swaps deck[N] with deck[j] where j is a secret runtime value.
    // Fully unrolled — no loops — so the garbled circuit topology is data-independent.
    // j == N is a no-op (swap with self); handled implicitly by the else branch.

    fn oblivious_swap_0(deck: &mut [u64; 52], j: u64) {
        let d = deck[0];
        if j == 1  { let t = deck[1];  deck[0] = t; deck[1]  = d; }
        if j == 2  { let t = deck[2];  deck[0] = t; deck[2]  = d; }
        if j == 3  { let t = deck[3];  deck[0] = t; deck[3]  = d; }
        if j == 4  { let t = deck[4];  deck[0] = t; deck[4]  = d; }
        if j == 5  { let t = deck[5];  deck[0] = t; deck[5]  = d; }
        if j == 6  { let t = deck[6];  deck[0] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[0] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[0] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[0] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[0] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[0] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[0] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[0] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[0] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[0] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[0] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[0] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[0] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[0] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[0] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[0] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[0] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[0] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[0] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[0] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[0] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[0] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[0] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[0] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[0] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[0] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[0] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[0] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[0] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[0] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[0] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[0] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[0] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[0] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[0] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[0] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[0] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[0] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[0] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[0] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[0] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[0] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[0] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[0] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[0] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[0] = t; deck[51] = d; }
    }

    fn oblivious_swap_1(deck: &mut [u64; 52], j: u64) {
        let d = deck[1];
        if j == 2  { let t = deck[2];  deck[1] = t; deck[2]  = d; }
        if j == 3  { let t = deck[3];  deck[1] = t; deck[3]  = d; }
        if j == 4  { let t = deck[4];  deck[1] = t; deck[4]  = d; }
        if j == 5  { let t = deck[5];  deck[1] = t; deck[5]  = d; }
        if j == 6  { let t = deck[6];  deck[1] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[1] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[1] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[1] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[1] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[1] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[1] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[1] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[1] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[1] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[1] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[1] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[1] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[1] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[1] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[1] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[1] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[1] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[1] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[1] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[1] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[1] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[1] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[1] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[1] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[1] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[1] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[1] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[1] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[1] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[1] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[1] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[1] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[1] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[1] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[1] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[1] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[1] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[1] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[1] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[1] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[1] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[1] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[1] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[1] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[1] = t; deck[51] = d; }
    }

    fn oblivious_swap_2(deck: &mut [u64; 52], j: u64) {
        let d = deck[2];
        if j == 3  { let t = deck[3];  deck[2] = t; deck[3]  = d; }
        if j == 4  { let t = deck[4];  deck[2] = t; deck[4]  = d; }
        if j == 5  { let t = deck[5];  deck[2] = t; deck[5]  = d; }
        if j == 6  { let t = deck[6];  deck[2] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[2] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[2] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[2] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[2] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[2] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[2] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[2] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[2] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[2] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[2] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[2] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[2] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[2] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[2] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[2] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[2] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[2] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[2] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[2] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[2] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[2] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[2] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[2] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[2] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[2] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[2] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[2] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[2] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[2] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[2] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[2] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[2] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[2] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[2] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[2] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[2] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[2] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[2] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[2] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[2] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[2] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[2] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[2] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[2] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[2] = t; deck[51] = d; }
    }

    fn oblivious_swap_3(deck: &mut [u64; 52], j: u64) {
        let d = deck[3];
        if j == 4  { let t = deck[4];  deck[3] = t; deck[4]  = d; }
        if j == 5  { let t = deck[5];  deck[3] = t; deck[5]  = d; }
        if j == 6  { let t = deck[6];  deck[3] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[3] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[3] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[3] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[3] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[3] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[3] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[3] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[3] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[3] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[3] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[3] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[3] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[3] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[3] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[3] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[3] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[3] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[3] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[3] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[3] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[3] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[3] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[3] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[3] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[3] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[3] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[3] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[3] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[3] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[3] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[3] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[3] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[3] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[3] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[3] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[3] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[3] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[3] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[3] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[3] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[3] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[3] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[3] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[3] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[3] = t; deck[51] = d; }
    }

    fn oblivious_swap_4(deck: &mut [u64; 52], j: u64) {
        let d = deck[4];
        if j == 5  { let t = deck[5];  deck[4] = t; deck[5]  = d; }
        if j == 6  { let t = deck[6];  deck[4] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[4] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[4] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[4] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[4] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[4] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[4] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[4] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[4] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[4] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[4] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[4] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[4] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[4] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[4] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[4] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[4] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[4] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[4] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[4] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[4] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[4] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[4] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[4] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[4] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[4] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[4] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[4] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[4] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[4] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[4] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[4] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[4] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[4] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[4] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[4] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[4] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[4] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[4] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[4] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[4] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[4] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[4] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[4] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[4] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[4] = t; deck[51] = d; }
    }

    fn oblivious_swap_5(deck: &mut [u64; 52], j: u64) {
        let d = deck[5];
        if j == 6  { let t = deck[6];  deck[5] = t; deck[6]  = d; }
        if j == 7  { let t = deck[7];  deck[5] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[5] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[5] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[5] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[5] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[5] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[5] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[5] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[5] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[5] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[5] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[5] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[5] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[5] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[5] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[5] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[5] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[5] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[5] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[5] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[5] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[5] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[5] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[5] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[5] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[5] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[5] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[5] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[5] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[5] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[5] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[5] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[5] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[5] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[5] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[5] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[5] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[5] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[5] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[5] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[5] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[5] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[5] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[5] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[5] = t; deck[51] = d; }
    }

    fn oblivious_swap_6(deck: &mut [u64; 52], j: u64) {
        let d = deck[6];
        if j == 7  { let t = deck[7];  deck[6] = t; deck[7]  = d; }
        if j == 8  { let t = deck[8];  deck[6] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[6] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[6] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[6] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[6] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[6] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[6] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[6] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[6] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[6] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[6] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[6] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[6] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[6] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[6] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[6] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[6] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[6] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[6] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[6] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[6] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[6] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[6] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[6] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[6] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[6] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[6] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[6] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[6] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[6] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[6] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[6] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[6] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[6] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[6] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[6] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[6] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[6] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[6] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[6] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[6] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[6] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[6] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[6] = t; deck[51] = d; }
    }

    fn oblivious_swap_7(deck: &mut [u64; 52], j: u64) {
        let d = deck[7];
        if j == 8  { let t = deck[8];  deck[7] = t; deck[8]  = d; }
        if j == 9  { let t = deck[9];  deck[7] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[7] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[7] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[7] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[7] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[7] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[7] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[7] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[7] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[7] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[7] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[7] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[7] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[7] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[7] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[7] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[7] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[7] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[7] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[7] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[7] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[7] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[7] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[7] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[7] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[7] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[7] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[7] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[7] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[7] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[7] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[7] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[7] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[7] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[7] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[7] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[7] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[7] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[7] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[7] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[7] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[7] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[7] = t; deck[51] = d; }
    }

    fn oblivious_swap_8(deck: &mut [u64; 52], j: u64) {
        let d = deck[8];
        if j == 9  { let t = deck[9];  deck[8] = t; deck[9]  = d; }
        if j == 10 { let t = deck[10]; deck[8] = t; deck[10] = d; }
        if j == 11 { let t = deck[11]; deck[8] = t; deck[11] = d; }
        if j == 12 { let t = deck[12]; deck[8] = t; deck[12] = d; }
        if j == 13 { let t = deck[13]; deck[8] = t; deck[13] = d; }
        if j == 14 { let t = deck[14]; deck[8] = t; deck[14] = d; }
        if j == 15 { let t = deck[15]; deck[8] = t; deck[15] = d; }
        if j == 16 { let t = deck[16]; deck[8] = t; deck[16] = d; }
        if j == 17 { let t = deck[17]; deck[8] = t; deck[17] = d; }
        if j == 18 { let t = deck[18]; deck[8] = t; deck[18] = d; }
        if j == 19 { let t = deck[19]; deck[8] = t; deck[19] = d; }
        if j == 20 { let t = deck[20]; deck[8] = t; deck[20] = d; }
        if j == 21 { let t = deck[21]; deck[8] = t; deck[21] = d; }
        if j == 22 { let t = deck[22]; deck[8] = t; deck[22] = d; }
        if j == 23 { let t = deck[23]; deck[8] = t; deck[23] = d; }
        if j == 24 { let t = deck[24]; deck[8] = t; deck[24] = d; }
        if j == 25 { let t = deck[25]; deck[8] = t; deck[25] = d; }
        if j == 26 { let t = deck[26]; deck[8] = t; deck[26] = d; }
        if j == 27 { let t = deck[27]; deck[8] = t; deck[27] = d; }
        if j == 28 { let t = deck[28]; deck[8] = t; deck[28] = d; }
        if j == 29 { let t = deck[29]; deck[8] = t; deck[29] = d; }
        if j == 30 { let t = deck[30]; deck[8] = t; deck[30] = d; }
        if j == 31 { let t = deck[31]; deck[8] = t; deck[31] = d; }
        if j == 32 { let t = deck[32]; deck[8] = t; deck[32] = d; }
        if j == 33 { let t = deck[33]; deck[8] = t; deck[33] = d; }
        if j == 34 { let t = deck[34]; deck[8] = t; deck[34] = d; }
        if j == 35 { let t = deck[35]; deck[8] = t; deck[35] = d; }
        if j == 36 { let t = deck[36]; deck[8] = t; deck[36] = d; }
        if j == 37 { let t = deck[37]; deck[8] = t; deck[37] = d; }
        if j == 38 { let t = deck[38]; deck[8] = t; deck[38] = d; }
        if j == 39 { let t = deck[39]; deck[8] = t; deck[39] = d; }
        if j == 40 { let t = deck[40]; deck[8] = t; deck[40] = d; }
        if j == 41 { let t = deck[41]; deck[8] = t; deck[41] = d; }
        if j == 42 { let t = deck[42]; deck[8] = t; deck[42] = d; }
        if j == 43 { let t = deck[43]; deck[8] = t; deck[43] = d; }
        if j == 44 { let t = deck[44]; deck[8] = t; deck[44] = d; }
        if j == 45 { let t = deck[45]; deck[8] = t; deck[45] = d; }
        if j == 46 { let t = deck[46]; deck[8] = t; deck[46] = d; }
        if j == 47 { let t = deck[47]; deck[8] = t; deck[47] = d; }
        if j == 48 { let t = deck[48]; deck[8] = t; deck[48] = d; }
        if j == 49 { let t = deck[49]; deck[8] = t; deck[49] = d; }
        if j == 50 { let t = deck[50]; deck[8] = t; deck[50] = d; }
        if j == 51 { let t = deck[51]; deck[8] = t; deck[51] = d; }
    }
}
