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
        // We use a 52-element deck represented as an array of indices 0-51.
        // The Fisher-Yates shuffle is performed using oblivious conditional swaps
        // implemented through exhaustive if-equality checks — each swap inspects
        // all 52 positions to obliviously select the target without leaking the
        // random index. We only shuffle the first 9 positions (4 hole + 5 community).

        let mut deck = [
             0u64,  1,  2,  3,  4,  5,  6,  7,  8,  9,
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
            40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
            50, 51,
        ];

        // LCG constants (Knuth MMIX): multiplier and increment.
        const A: u64 = 6_364_136_223_846_793_005;
        const C: u64 = 1_442_695_040_888_963_407;

        // Macro-style inlining: generate 9 LCG steps and oblivious swaps.
        // Each swap(deck, i, j) where j is data-dependent uses a linear scan
        // over all 52 positions to obliviously select and exchange elements.

        let r0 = inp.seed.wrapping_mul(A).wrapping_add(C);
        let j0 = (r0 >> 33) % 52u64;
        oblivious_swap_0(&mut deck, j0);

        let r1 = r0.wrapping_mul(A).wrapping_add(C);
        let j1 = (r1 >> 33) % 51u64;
        oblivious_swap_1(&mut deck, j1);

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
        [
            deck[0] ^ inp.p1_mask,
            deck[1] ^ inp.p1_mask,
            deck[2] ^ inp.p2_mask,
            deck[3] ^ inp.p2_mask,
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
    // Each function swaps deck[position] with deck[j] where j is a runtime value.
    // The scan over all valid target positions ensures the circuit topology is
    // data-independent — required for garbled circuit privacy.

    fn oblivious_swap_0(deck: &mut [u64; 52], j: u64) {
        let d0 = deck[0];
        let mut k = 0usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[0] = dk;
                deck[k] = d0;
            }
            k += 1;
        }
    }

    fn oblivious_swap_1(deck: &mut [u64; 52], j: u64) {
        let d1 = deck[1];
        let mut k = 1usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[1] = dk;
                deck[k] = d1;
            }
            k += 1;
        }
    }

    fn oblivious_swap_2(deck: &mut [u64; 52], j: u64) {
        let d2 = deck[2];
        let mut k = 2usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[2] = dk;
                deck[k] = d2;
            }
            k += 1;
        }
    }

    fn oblivious_swap_3(deck: &mut [u64; 52], j: u64) {
        let d3 = deck[3];
        let mut k = 3usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[3] = dk;
                deck[k] = d3;
            }
            k += 1;
        }
    }

    fn oblivious_swap_4(deck: &mut [u64; 52], j: u64) {
        let d4 = deck[4];
        let mut k = 4usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[4] = dk;
                deck[k] = d4;
            }
            k += 1;
        }
    }

    fn oblivious_swap_5(deck: &mut [u64; 52], j: u64) {
        let d5 = deck[5];
        let mut k = 5usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[5] = dk;
                deck[k] = d5;
            }
            k += 1;
        }
    }

    fn oblivious_swap_6(deck: &mut [u64; 52], j: u64) {
        let d6 = deck[6];
        let mut k = 6usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[6] = dk;
                deck[k] = d6;
            }
            k += 1;
        }
    }

    fn oblivious_swap_7(deck: &mut [u64; 52], j: u64) {
        let d7 = deck[7];
        let mut k = 7usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[7] = dk;
                deck[k] = d7;
            }
            k += 1;
        }
    }

    fn oblivious_swap_8(deck: &mut [u64; 52], j: u64) {
        let d8 = deck[8];
        let mut k = 8usize;
        while k < 52 {
            let dk = deck[k];
            if j == k as u64 {
                deck[8] = dk;
                deck[k] = d8;
            }
            k += 1;
        }
    }
}
