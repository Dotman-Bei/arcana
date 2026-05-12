/**
 * Register Arcium computation definitions for Arcana Hold'em.
 *
 * Run AFTER:
 *   1. anchor build        → generates target/idl/arcana_holdem.json and deal_cards.arcis
 *   2. anchor deploy       → program live on devnet
 *   3. Upload deal_cards.arcis to Supabase (public bucket) → get URL
 *   4. arcium init-mxe -k ~/.config/solana/id.json -p <PROGRAM_ID> -f 456 -r 4 -u <RPC_URL>
 *   5. arcium mxe-info <PROGRAM_ID> -u devnet  (verify Status: Active + keys present)
 *
 * Usage:
 *   node scripts/init-comp-defs.mjs \
 *     --keypair ~/.config/solana/id.json \
 *     --program-id <DEPLOYED_PROGRAM_ID> \
 *     --arcis-url https://your-supabase.supabase.co/storage/v1/object/public/arcana/deal_cards.arcis
 *
 * After running, verify:
 *   arcium mxe-info <PROGRAM_ID> -u devnet
 *   (You should see the deal_cards computation definition offset listed)
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { AnchorProvider, Program, setProvider } from "@coral-xyz/anchor";
import { ARCIUM_ADDR, getMXEAccAddress } from "@arcium-hq/client";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

// ── Parse CLI args ────────────────────────────────────────────────────────────
const parseArgs = (argv) => {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) { opts[argv[i].slice(2)] = argv[i + 1]; i++; }
  }
  return opts;
};

const opts = parseArgs(process.argv.slice(2));
if (!opts.keypair || !opts["program-id"] || !opts["arcis-url"]) {
  console.error(`Usage: node scripts/init-comp-defs.mjs \\
  --keypair ~/.config/solana/id.json \\
  --program-id <DEPLOYED_PROGRAM_ID> \\
  --arcis-url <PUBLIC_URL_OF_DEAL_CARDS.ARCIS>`);
  process.exit(1);
}

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";
const kpPath  = opts.keypair.replace(/^~/, process.env.USERPROFILE || process.env.HOME);

const OUR_PROGRAM_ID    = new PublicKey(opts["program-id"]);
const ARCIUM_PROGRAM_ID = new PublicKey(ARCIUM_ADDR);
const MXE_PUBKEY        = getMXEAccAddress(OUR_PROGRAM_ID);
const ARCIS_URL         = opts["arcis-url"];

// comp_def_offset("deal_cards") — SHA-256("deal_cards") first 4 bytes LE
// Precomputed: 0xaf840597
const COMP_DEF_OFFSET = 0xaf840597;
const COMP_DEF_PUBKEY = PublicKey.findProgramAddressSync(
  [Buffer.from("ComputationDefinitionAccount"), Buffer.from(new Uint32Array([COMP_DEF_OFFSET]).buffer)],
  ARCIUM_PROGRAM_ID,
)[0];

// ── Main ──────────────────────────────────────────────────────────────────────
async function main() {
  const wallet = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(kpPath, "utf8"))),
  );
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Wallet:     ", wallet.publicKey.toBase58());
  console.log("Program:    ", OUR_PROGRAM_ID.toBase58());
  console.log("MXE:        ", MXE_PUBKEY.toBase58());
  console.log("CompDef PDA:", COMP_DEF_PUBKEY.toBase58());
  console.log("ARCIS URL:  ", ARCIS_URL);
  console.log("Balance:    ", (await connection.getBalance(wallet.publicKey)) / 1e9, "SOL\n");

  // Verify MXE is active before proceeding.
  const mxeInfo = await connection.getAccountInfo(MXE_PUBKEY);
  if (!mxeInfo) {
    console.error("✗ MXE account not found. Run first:");
    console.error(`  arcium init-mxe -k ${opts.keypair} -p ${OUR_PROGRAM_ID.toBase58()} -f 456 -r 4 -u ${RPC_URL}`);
    process.exit(1);
  }
  console.log("✓ MXE account found");

  // Verify CompDef is not already initialized.
  const existing = await connection.getAccountInfo(COMP_DEF_PUBKEY);
  if (existing) {
    console.log("✓ CompDef already initialized at:", COMP_DEF_PUBKEY.toBase58());
    console.log("  Nothing to do. Run: arcium mxe-info", OUR_PROGRAM_ID.toBase58(), "-u devnet");
    return;
  }

  const idlPath = path.join(process.cwd(), "target", "idl", "arcana_holdem.json");
  if (!fs.existsSync(idlPath)) {
    console.error("IDL not found at", idlPath, "\nRun: anchor build");
    process.exit(1);
  }
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));

  const provider = new AnchorProvider(
    connection,
    {
      publicKey: wallet.publicKey,
      signTransaction: async (tx) => { tx.partialSign(wallet); return tx; },
      signAllTransactions: async (txs) => txs.map(tx => { tx.partialSign(wallet); return tx; }),
    },
    { commitment: "confirmed" },
  );
  setProvider(provider);
  const program = new Program(idl, provider);

  console.log("Registering deal_cards computation definition…");
  try {
    const tx = await program.methods
      .initDealCardsCompDef(ARCIS_URL)
      .accounts({
        payer:          wallet.publicKey,
        mxeAccount:     MXE_PUBKEY,
        compDefAccount: COMP_DEF_PUBKEY,
        arciumProgram:  ARCIUM_PROGRAM_ID,
        systemProgram:  SystemProgram.programId,
      })
      .signers([wallet])
      .rpc();

    console.log("✓ init_deal_cards_comp_def tx:", tx);
    console.log("\nNext — verify computation definition is listed:");
    console.log(`  arcium mxe-info ${OUR_PROGRAM_ID.toBase58()} -u devnet`);
    console.log("\nExpected output should include:");
    console.log("  Computation definitions:");
    console.log("    deal_cards  offset=0xaf840597  url=" + ARCIS_URL);
  } catch (e) {
    console.error("✗ Error:", e.message);
    if (e.logs) console.error(e.logs.join("\n"));
    process.exit(1);
  }
}

main().catch(e => { console.error(e.message); process.exit(1); });
