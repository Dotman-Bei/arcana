# Arcana — Arcium MXE Integration

## What is this?

Arcana Hold'em is provably fair Texas Hold'em on Solana. There is no trusted dealer, no server, and no way for any player to know their opponent's hole cards until showdown.

This is made possible by Arcium's Multi-party eXecution Environment (MXE), which runs garbled circuits inside a secure multi-party computation cluster. The circuit shuffles the deck and deals cards entirely within the MXE — neither player, the Solana chain, nor any MXE node ever sees the plaintext hole cards during the deal.

---

## Where Arcium is Used

### 1. `deal_cards` — The Core Privacy Circuit

**File:** `arcium_compute/src/lib.rs`

**Input:** `Enc<Shared, DeckInput>` — three `u64` values encrypted jointly under the MXE cluster's X25519 key:
- `seed` — combined deck entropy: `salt_a XOR salt_b` (neither player controls the seed alone)
- `p1_mask` — Player A's secret XOR mask (only A knows this)
- `p2_mask` — Player B's secret XOR mask (only B knows this)

**What the MXE does inside the garbled circuit:**
1. Initialises a 52-card deck (values 0-51)
2. Performs a Fisher-Yates shuffle using the combined seed and oblivious conditional swaps (circuit-topology independent of card values — garbled circuit privacy property)
3. Assigns deck[0..1] to Player A and deck[2..3] to Player B
4. Masks hole cards: `card_a1 ^ p1_mask`, `card_a2 ^ p1_mask`, `card_b1 ^ p2_mask`, `card_b2 ^ p2_mask`
5. Returns community cards (deck[4..8]) in plaintext — they are public information released by the contract progressively at each street

**Output:** `[u64; 9].reveal()` — the nine values are written on-chain by the `deal_callback` instruction.

**Privacy guarantee:**
- Player A can unmask their cards locally: `card = masked_value XOR p1_mask`
- Player B cannot unmask Player A's cards (different mask)
- Nobody outside the MXE ever sees `p1_mask`, `p2_mask`, or the plaintext hole cards
- The garbled circuit topology is data-independent: the shuffle path cannot be inferred from circuit execution timing

### 2. On-chain Mask Commitment

**File:** `programs/arcana_holdem/src/instructions/reveal_showdown.rs`

Before the game starts, each player commits to their mask:
```
commit = SHA-256(mask_le_bytes || table_pubkey_bytes)
```

This is stored in `Table.player_a_mask_commit` / `Table.player_b_mask_commit`. At showdown, the contract verifies the revealed mask against this commitment before unmasking cards, preventing a player from lying about their mask.

### 3. Client-side Encryption

**File:** `src/lib/arcium-encrypt.ts`

Both players encrypt the `DeckInput` struct fields for the MXE using:
1. Ephemeral X25519 keypair
2. X25519 ECDH with MXE cluster pubkey → 32-byte shared secret
3. HKDF-SHA-256(secret, `"arcium-enc-shared-v1"`) → AES-256 key
4. AES-256-GCM per field with per-field nonce

This matches the `Enc<Shared, T>` scheme expected by arcium-anchor 0.9.6.

---

## Why Arcium is Necessary

Without MPC, the only alternatives for provably fair poker are:

| Approach | Problem |
|---|---|
| Trusted dealer server | Single point of failure — can collude or be hacked |
| ZK proofs for shuffling | Prover time is O(n log n) per shuffle; impractical for real-time games |
| Commit-reveal | Either requires a trusted timelock or leaks cards if a player times out |
| Homomorphic encryption | Too slow for interactive games; card evaluation in HE is prohibitive |

Arcium's garbled circuit evaluation runs the shuffle and masking in microseconds once scheduled on the cluster, with cryptographic guarantees equivalent to secure 2PC. The MXE is the first system that makes provably fair card dealing on Solana both fast and practical.

---

## Game Flow

```
Player A: init_table  (deposits buy-in, posts SB)
Player B: join_table  (deposits buy-in, posts BB, submits encrypted DeckInput)
          → Arcium queues deal_cards computation
          → MXE runs garbled circuit, calls deal_callback
          → Table state: PreFlop (cards stored masked on-chain)

Each player unmasks their own hole cards locally using their secret mask.

Betting rounds: submit_action (Fold / Check / Call / Raise)
  PreFlop → Flop (3 community cards active)
  Flop    → Turn (4th card active)
  Turn    → River (5th card active)
  River   → Showdown

Showdown: each player calls reveal_showdown(mask)
  → Contract verifies mask against commitment
  → Unmasks cards
  → Evaluates best 5-of-7 hand
  → Transfers pot to winner
```

---

## Deployment Steps

```bash
# 1. Build the Anchor program and Arcium circuit
anchor build

# 2. Deploy to Solana devnet
anchor deploy --provider.cluster devnet

# 3. Register your MXE and configure cluster
arcium register-mxe --cluster devnet
arcium info --cluster devnet  # note the X25519 pubkey and PDA addresses

# 4. Initialize the deal_cards computation definition
node scripts/init-arcium-onchain.mjs --keypair ~/.config/solana/id.json --step init-comp-def

# 5. Copy .env.example to .env and populate VITE_* variables

# 6. Run the frontend
npm run dev
```

---

## v2 Roadmap

- **Private showdown** — `evaluate_showdown` MXE circuit: unmask both hands inside the garbled circuit, return only winner ID. Losing hand is never revealed on-chain.
- **Multi-table tournaments** — multiple concurrent tables with a tournament bracket contract
- **Rake mechanism** — small percentage of each pot sent to a treasury account
- **Full 6-player game** — extend `DeckInput` to handle N players via Arcium's N-party MXE
