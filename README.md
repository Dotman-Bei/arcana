# Arcana Hold'em

Provably fair Texas Hold'em on Solana. Hole cards are dealt privately via the **Arcium MXE** garbled circuit — nobody sees your cards, not even the blockchain.

Built for the [Arcium RTG — Hidden-Information Games](https://rtg.arcium.com/rtg/dev-hidden-games).

## How it works

1. Player A creates a table and commits to a secret XOR mask.
2. Player B joins and submits an Arcium MXE computation that shuffles the deck inside a garbled circuit, masks each player's hole cards with their respective secrets, and returns the result on-chain.
3. Players bet through Pre-Flop → Flop → Turn → River using standard Hold'em rules.
4. At Showdown, each player reveals their mask. The on-chain program unmasks the cards, evaluates best 5-of-7 hands, and pays the pot to the winner.

See [ARCIUM.md](./ARCIUM.md) for the full privacy architecture.

---

## Build & Deploy — Full Workflow

Requires: **Rust**, **Anchor 0.32.1**, **Solana CLI**, **Arcium CLI**, **Node.js ≥ 20**

### Step 1 — Build Anchor program + Arcium circuit

```bash
anchor build
```

This produces:
- `target/deploy/arcana_holdem.so` — Solana program binary
- `target/deploy/arcana_holdem-keypair.json` — program keypair
- `target/idl/arcana_holdem.json` — Anchor IDL
- `arcium_compute/deal_cards.arcis` — compiled Arcium garbled circuit

### Step 2 — Get the real program ID

```bash
anchor keys list
# → arcana_holdem: <REAL_PROGRAM_ID>
```

Update `declare_id!` in [programs/arcana_holdem/src/lib.rs](programs/arcana_holdem/src/lib.rs) and `[programs.devnet]` in [Anchor.toml](Anchor.toml) with this ID.
Rebuild: `anchor build`

### Step 3 — Confirm the program ID keypair

```bash
solana address -k target/deploy/arcana_holdem-keypair.json
```

### Step 4 — Deploy to Solana devnet

```bash
anchor deploy --provider.cluster devnet
```

Check it's live:
```bash
solana program show <PROGRAM_ID> -u devnet
```

### Step 5 — Upload deal_cards.arcis to Supabase

1. Create a Supabase project → Storage → New bucket (name: `arcana`, public: true)
2. Upload `arcium_compute/deal_cards.arcis` to the bucket
3. Copy the public URL — looks like:
   ```
   https://<project>.supabase.co/storage/v1/object/public/arcana/deal_cards.arcis
   ```

> **Why:** Arcium MXE nodes fetch this file to know which garbled circuit to execute for each `deal_cards` computation. Without the URL registered on-chain, the MXE cannot process deal requests.

### Step 6 — Initialize MXE for your program

```bash
RPC_URL="https://api.devnet.solana.com"
# (Recommended: use a Helius free RPC for reliability)
# RPC_URL="https://devnet.helius-rpc.com/?api-key=<YOUR_KEY>"

arcium init-mxe \
  -k ~/.config/solana/id.json \
  -p <PROGRAM_ID> \
  -f 456 \
  -r 4 \
  -u "$RPC_URL"
```

- `-f 456` — Arcium devnet cluster offset
- `-r 4` — recovery size

Verify the MXE is active:
```bash
arcium mxe-info <PROGRAM_ID> -u devnet
```

You want to see `Status: Active` and all four keys present (X25519, Ed25519, ElGamal, BLS).

### Step 7 — Register the deal_cards computation definition

```bash
export NODE_OPTIONS="--dns-result-order=ipv4first"
export ANCHOR_PROVIDER_URL="$RPC_URL"

node scripts/init-comp-defs.mjs \
  --keypair ~/.config/solana/id.json \
  --program-id <PROGRAM_ID> \
  --arcis-url <SUPABASE_URL_FROM_STEP_5>
```

Or via npm:
```bash
node scripts/init-comp-defs.mjs --keypair ~/.config/solana/id.json --program-id <PROGRAM_ID> --arcis-url <URL>
```

Verify the computation definition is listed:
```bash
arcium mxe-info <PROGRAM_ID> -u devnet
# Should show: deal_cards  offset=0xaf840597
```

### Step 8 — Configure the frontend

```bash
cp .env.example .env
# Fill in VITE_PROGRAM_ID and all VITE_ARCIUM_* values from arcium mxe-info output
```

Wire in the IDL — in [src/App.tsx](src/App.tsx) line 25, replace:
```ts
const IDL = {} as object;
```
with:
```ts
import IDL from "../../target/idl/arcana_holdem.json";
```

### Step 9 — Run the frontend

```bash
npm install
npm run dev
```

---

## What is private (Arcium MXE)?

| Data | Private? | Mechanism |
|---|---|---|
| Player hole cards | ✅ Yes | Masked with player's secret XOR key inside MXE circuit |
| Deck shuffle order | ✅ Yes | Fisher-Yates inside garbled circuit, never revealed |
| Opponent's mask | ✅ Yes | Each player's mask is only known to them |
| Community cards | ❌ Public | Released by contract per street (flop/turn/river) |
| Bets / pot | ❌ Public | Standard Solana on-chain state |
| Final hands at showdown | ❌ Public | Players reveal masks; contract evaluates on-chain |

---

## Project structure

```
programs/arcana_holdem/   Anchor on-chain program (Rust)
  src/lib.rs              Table PDA state, instruction declarations
  src/instructions/
    init_table            Player A creates table + commits mask
    join_table            Player B joins + fires Arcium MXE deal
    deal_callback         Arcium callback — stores masked hole cards
    submit_action         Fold / Check / Call / Raise
    reveal_showdown       Mask reveal + hand eval + pot settlement
    close_table           Reclaim rent after game settles

arcium_compute/src/lib.rs Arcium garbled circuit
  deal_cards              Fisher-Yates shuffle + XOR hole-card masking

scripts/
  init-comp-defs.mjs      Register deal_cards.arcis computation definition
  init-arcium-onchain.mjs Legacy step-by-step MXE setup helper

src/
  lib/
    arcium-config.ts      PDA derivation, env-var config
    arcium-accounts.ts    MXE/Mempool account decode
    arcium-encrypt.ts     X25519+HKDF+AES-GCM for Enc<Shared, T>
    holdem-client.ts      Anchor program interaction layer
    poker.ts              Card/hand display utilities
  components/
    PlayingCard           Single card face/back
    PlayerSeat            Player area with cards + stack
    CommunityCards        Board cards with street gating
    BettingPanel          Fold/Check/Call/Raise controls
    GameLog               Live event log
  App.tsx                 Main game UI and state machine
```
