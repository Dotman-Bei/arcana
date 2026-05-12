/**
 * One-time Arcium on-chain setup for Arcana Hold'em.
 *
 * Usage:
 *   node scripts/init-arcium-onchain.mjs --keypair ~/.config/solana/id.json --step check
 *   node scripts/init-arcium-onchain.mjs --keypair ~/.config/solana/id.json --step set-cluster
 *   node scripts/init-arcium-onchain.mjs --keypair ~/.config/solana/id.json --step requeue-keygen
 *   node scripts/init-arcium-onchain.mjs --keypair ~/.config/solana/id.json --step init-comp-def
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  SystemProgram,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { AnchorProvider, Program, setProvider } from "@coral-xyz/anchor";
import {
  ARCIUM_IDL,
  ARCIUM_ADDR,
  getMXEAccAddress,
  getClusterAccAddress,
  getMempoolAccAddress,
  getExecutingPoolAccAddress,
} from "@arcium-hq/client";
import fs from "node:fs";
import path from "node:path";

const OUR_PROGRAM_ID    = new PublicKey(process.env.PROGRAM_ID || "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
const ARCIUM_PROGRAM_ID = new PublicKey(ARCIUM_ADDR);
const CLUSTER_OFFSET    = 456;

const MXE_PUBKEY   = getMXEAccAddress(OUR_PROGRAM_ID);
const CLUSTER_456  = getClusterAccAddress(CLUSTER_OFFSET, ARCIUM_PROGRAM_ID);
const MEMPOOL_456  = getMempoolAccAddress(CLUSTER_OFFSET, ARCIUM_PROGRAM_ID);
const EXECPOOL_456 = getExecutingPoolAccAddress(CLUSTER_OFFSET, ARCIUM_PROGRAM_ID);

// comp_def_offset("deal_cards") — first 4 bytes of SHA-256("deal_cards") as LE u32
// SHA-256("deal_cards") = 970584af3a14a74cd7625bca6f53a863...
// First 4 bytes LE = 0xaf840597 = 2944664983
const COMP_DEF_OFFSET = 0xaf840597;
const COMP_DEF_PUBKEY = PublicKey.findProgramAddressSync(
  [Buffer.from("ComputationDefinitionAccount"), Buffer.from(new Uint32Array([COMP_DEF_OFFSET]).buffer)],
  ARCIUM_PROGRAM_ID,
)[0];

const parseArgs = (argv) => {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) { opts[argv[i].slice(2)] = argv[i + 1]; i++; }
  }
  return opts;
};

const parseMxeX25519 = (data) => {
  let pos = 8;
  const hasCluster = data[pos] === 1; pos += 1;
  if (hasCluster) pos += 4;
  pos += 8; pos += 8; pos += 32;
  const hasAuth = data[pos] === 1; pos += 1;
  if (hasAuth) pos += 32;
  const isSet = data[pos] === 1; pos += 1;
  if (!isSet) return null;
  const x25519 = data.slice(pos, pos + 32);
  return x25519.every(b => b === 0) ? null : x25519;
};

const parseMxeKeygenOffset = (data) => {
  let pos = 8;
  const hasCluster = data[pos] === 1; pos += 1;
  if (hasCluster) pos += 4;
  return data.readBigUInt64LE(pos);
};

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (!opts.keypair) {
    console.error("Usage: node scripts/init-arcium-onchain.mjs --keypair <path> --step <check|set-cluster|requeue-keygen|init-comp-def>");
    process.exit(1);
  }

  const kpPath = opts.keypair.replace(/^~/, process.env.USERPROFILE || process.env.HOME);
  const wallet = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(kpPath, "utf8"))));
  const connection = new Connection("https://api.devnet.solana.com", "confirmed");

  console.log("Wallet:", wallet.publicKey.toBase58());
  console.log("Balance:", (await connection.getBalance(wallet.publicKey)) / 1e9, "SOL");

  const step = opts.step || "check";

  if (step === "check") {
    console.log("\n=== Arcium MXE Status ===");
    console.log("MXE:", MXE_PUBKEY.toBase58());
    const mxeInfo = await connection.getAccountInfo(MXE_PUBKEY);
    if (!mxeInfo) { console.log("MXE account NOT FOUND"); return; }
    const x25519 = parseMxeX25519(mxeInfo.data);
    console.log(x25519 ? `✓ x25519: ${Buffer.from(x25519).toString("hex")}` : "✗ x25519 NOT SET");
    console.log("CompDef:", COMP_DEF_PUBKEY.toBase58());
    const compDefInfo = await connection.getAccountInfo(COMP_DEF_PUBKEY);
    console.log(compDefInfo ? "✓ CompDef EXISTS" : "✗ CompDef NOT initialized → run init-comp-def");
    return;
  }

  if (step === "set-cluster" || step === "requeue-keygen") {
    // MXE initialization is handled by the Arcium CLI, not this script.
    // Run this command instead:
    console.log("\nMXE initialization is done via the Arcium CLI:");
    console.log(`\n  arcium init-mxe \\`);
    console.log(`    -k ${opts.keypair} \\`);
    console.log(`    -p ${OUR_PROGRAM_ID.toBase58()} \\`);
    console.log(`    -f 456 \\`);
    console.log(`    -r 4 \\`);
    console.log(`    -u https://api.devnet.solana.com`);
    console.log(`\nThen verify:\n  arcium mxe-info ${OUR_PROGRAM_ID.toBase58()} -u devnet`);
    console.log("\nOnce MXE is Active, register the computation definition:");
    console.log(`  node scripts/init-comp-defs.mjs --keypair ${opts.keypair} --program-id ${OUR_PROGRAM_ID.toBase58()} --arcis-url <SUPABASE_URL>`);
    return;
  }

  if (step === "init-comp-def") {
    const mxeInfo = await connection.getAccountInfo(MXE_PUBKEY);
    if (!mxeInfo || !parseMxeX25519(mxeInfo.data)) {
      console.error("MXE x25519 key not set. Run requeue-keygen first.");
      process.exit(1);
    }
    const idlPath = path.join(process.cwd(), "target", "idl", "arcana_holdem.json");
    if (!fs.existsSync(idlPath)) {
      console.error("IDL not found at", idlPath, "\nRun: anchor build");
      process.exit(1);
    }
    const idl = JSON.parse(fs.readFileSync(idlPath, "utf8"));
    const provider = new AnchorProvider(connection, {
      publicKey: wallet.publicKey,
      signTransaction: async (tx) => { tx.partialSign(wallet); return tx; },
      signAllTransactions: async (txs) => txs.map(tx => { tx.partialSign(wallet); return tx; }),
    }, { commitment: "confirmed" });
    setProvider(provider);
    const program = new Program(idl, provider);
    if (!opts["arcis-url"]) {
      console.error("Missing --arcis-url. Upload deal_cards.arcis to Supabase and pass the public URL.");
      process.exit(1);
    }
    const arcisUrl = opts["arcis-url"];
    console.log("Registering deal_cards circuit URL:", arcisUrl);
    try {
      const tx = await program.methods.initDealCardsCompDef(arcisUrl)
        .accounts({ payer: wallet.publicKey, mxeAccount: MXE_PUBKEY, compDefAccount: COMP_DEF_PUBKEY, arciumProgram: ARCIUM_PROGRAM_ID, systemProgram: SystemProgram.programId })
        .signers([wallet])
        .rpc();
      console.log("✓ init_deal_cards_comp_def:", tx);
    } catch (e) {
      console.error("Error:", e.message);
      if (e.logs) console.error(e.logs.join("\n"));
    }
  }
}

main().catch(e => { console.error(e.message); process.exit(1); });
