/**
 * Arcana Hold'em on-chain client.
 * Wraps all Anchor program interactions for the React frontend.
 */

import { PublicKey, SystemProgram } from "@solana/web3.js";
import { AnchorProvider, BN, Program, setProvider } from "@coral-xyz/anchor";
import type { WalletContextState } from "@solana/wallet-adapter-react";
import {
  arciumMxeAccount,
  arciumMempoolAccount,
  arciumExecpoolAccount,
  arciumClusterAccount,
  arciumCompDefAccount,
  arciumFeePoolAccount,
  arciumClockAccount,
  arciumProgramId,
  deriveTablePda,
  deriveSignPda,
  deriveComputationAccount,
  programId,
} from "@/lib/arcium-config";
import { fetchNextComputationOffset, resolveClusterX25519Pubkey } from "@/lib/arcium-accounts";
import { encryptDeckInput, generatePlayerMask, computeMaskCommit } from "@/lib/arcium-encrypt";
import { solanaConnection } from "@/lib/solana";

export type TableAccount = {
  playerA: PublicKey;
  playerB: PublicKey;
  playerAStack: bigint;
  playerBStack: bigint;
  playerABet: bigint;
  playerBBet: bigint;
  pot: bigint;
  saltA: bigint;
  saltB: bigint;
  buyIn: bigint;
  bigBlind: bigint;
  playerACard1: bigint;
  playerACard2: bigint;
  playerBCard1: bigint;
  playerBCard2: bigint;
  community: number[];
  communityFlags: number;
  playerAMaskCommit: Uint8Array;
  playerBMaskCommit: Uint8Array;
  state: number;
  currentTurn: number;
  arciumTaskId: bigint;
  bump: number;
};

const TABLE_STATE_KEYS = [
  "waitingForPlayer", "dealing", "preFlop", "flop", "turn", "river", "showdown", "settled",
];

const makeProvider = (wallet: WalletContextState): AnchorProvider => {
  const p = new AnchorProvider(
    solanaConnection,
    { publicKey: wallet.publicKey!, signTransaction: wallet.signTransaction!, signAllTransactions: wallet.signAllTransactions! },
    { commitment: "confirmed" },
  );
  setProvider(p);
  return p;
};

/** Decode a raw Anchor-fetched Table account to our typed form. */
const decodeTable = (raw: any): TableAccount => ({
  playerA: raw.playerA,
  playerB: raw.playerB,
  playerAStack: BigInt(raw.playerAStack.toString()),
  playerBStack: BigInt(raw.playerBStack.toString()),
  playerABet: BigInt(raw.playerABet.toString()),
  playerBBet: BigInt(raw.playerBBet.toString()),
  pot: BigInt(raw.pot.toString()),
  saltA: BigInt(raw.saltA.toString()),
  saltB: BigInt(raw.saltB.toString()),
  buyIn: BigInt(raw.buyIn.toString()),
  bigBlind: BigInt(raw.bigBlind.toString()),
  playerACard1: BigInt(raw.playerACard1.toString()),
  playerACard2: BigInt(raw.playerACard2.toString()),
  playerBCard1: BigInt(raw.playerBCard1.toString()),
  playerBCard2: BigInt(raw.playerBCard2.toString()),
  community: Array.from(raw.community as number[]),
  communityFlags: raw.communityFlags,
  playerAMaskCommit: new Uint8Array(raw.playerAMaskCommit),
  playerBMaskCommit: new Uint8Array(raw.playerBMaskCommit),
  state: TABLE_STATE_KEYS.indexOf(Object.keys(raw.state).find(k => raw.state[k] !== undefined) ?? ""),
  currentTurn: raw.currentTurn,
  arciumTaskId: BigInt(raw.arciumTaskId.toString()),
  bump: raw.bump,
});

export const fetchTable = async (tablePda: PublicKey, idl: object): Promise<TableAccount | null> => {
  const provider = new AnchorProvider(solanaConnection, {} as any, {});
  const program = new Program(idl as any, provider);
  try {
    const raw = await (program.account as any)["table"].fetch(tablePda);
    return decodeTable(raw);
  } catch {
    return null;
  }
};

/**
 * Player A: create a new table.
 * Returns the table PDA, player A's secret hole-card mask, and the transaction signature.
 * Persist `mask` in localStorage — it is needed at showdown.
 */
export const initTable = async (
  wallet: WalletContextState,
  idl: object,
  buyInLamports: bigint,
  bigBlindLamports: bigint,
): Promise<{ tablePda: PublicKey; mask: bigint; sig: string }> => {
  const provider = makeProvider(wallet);
  const program = new Program(idl as any, provider);
  const playerA = wallet.publicKey!;

  const saltA = generatePlayerMask();
  const mask  = generatePlayerMask();
  const tablePda = deriveTablePda(playerA);

  // Compute mask commitment before the table exists so we can pass it in init_table.
  // We use the future table PDA address (deterministic from player A's pubkey).
  const commit = await computeMaskCommit(mask, tablePda.toBytes());
  const commitArr = Array.from(commit) as number[];

  const sig = await (program.methods as any)
    .initTable(
      new BN(buyInLamports.toString()),
      new BN(bigBlindLamports.toString()),
      new BN(saltA.toString()),
      commitArr,
    )
    .accounts({
      playerA,
      table: tablePda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  return { tablePda, mask, sig };
};

/**
 * Player B: join table and trigger the Arcium MXE deal.
 * `playerAMask` must be obtained from Player A out-of-band (e.g., shared via a lobby link or QR code).
 * Returns player B's secret mask and the transaction signature.
 */
export const joinTable = async (
  wallet: WalletContextState,
  idl: object,
  tablePda: PublicKey,
  playerAMask: bigint,
): Promise<{ mask: bigint; sig: string }> => {
  const provider = makeProvider(wallet);
  const program = new Program(idl as any, provider);
  const playerB = wallet.publicKey!;

  const saltB  = generatePlayerMask();
  const p2Mask = generatePlayerMask();

  const clusterKey = await resolveClusterX25519Pubkey(solanaConnection);
  if (!clusterKey) throw new Error("Arcium cluster X25519 key not found. Configure VITE_ARCIUM_CLUSTER_X25519_PUBKEY_HEX.");

  // Read salt_a from the Table PDA account (offset 112 after discriminator + fixed fields).
  // Layout: discrim(8) + playerA(32) + playerB(32) + a_stack(8) + b_stack(8) + a_bet(8) + b_bet(8) + pot(8) + saltA(8)
  const tableInfo = await solanaConnection.getAccountInfo(tablePda);
  if (!tableInfo) throw new Error("Table account not found on-chain");
  const saltA = new DataView(tableInfo.data.buffer, tableInfo.data.byteOffset + 112, 8).getBigUint64(0, true);

  const seed = saltA ^ saltB;

  const enc = await encryptDeckInput(seed, playerAMask, p2Mask, clusterKey);
  const computationOffset = await fetchNextComputationOffset(solanaConnection);

  const maskCommitB = await computeMaskCommit(p2Mask, tablePda.toBytes());

  const sig = await (program.methods as any)
    .joinTable(
      new BN(saltB.toString()),
      Array.from(maskCommitB),
      new BN(computationOffset.toString()),
      Array.from(enc.pubkey),
      new BN(enc.nonce.toString()),
      Array.from(enc.seedEnc),
      Array.from(enc.p1MaskEnc),
      Array.from(enc.p2MaskEnc),
    )
    .accounts({
      playerB,
      table: tablePda,
      signPdaAccount: deriveSignPda(),
      mxeAccount: arciumMxeAccount,
      mempoolAccount: arciumMempoolAccount,
      executingPool: arciumExecpoolAccount,
      computationAccount: deriveComputationAccount(computationOffset),
      compDefAccount: arciumCompDefAccount,
      clusterAccount: arciumClusterAccount,
      poolAccount: arciumFeePoolAccount,
      clockAccount: arciumClockAccount,
      systemProgram: SystemProgram.programId,
      arciumProgram: arciumProgramId,
    })
    .rpc();

  return { mask: p2Mask, sig };
};

/** Submit a betting action. `winnerWallet` is only used for the Fold action. */
export const submitAction = async (
  wallet: WalletContextState,
  idl: object,
  tablePda: PublicKey,
  action: "Fold" | "Check" | "Call" | "Raise",
  raiseTo: bigint,
  winnerWallet: PublicKey,
): Promise<string> => {
  const provider = makeProvider(wallet);
  const program = new Program(idl as any, provider);
  const actionEnum = { [action.toLowerCase()]: {} };

  return (program.methods as any)
    .submitAction(actionEnum, new BN(raiseTo.toString()))
    .accounts({ player: wallet.publicKey!, table: tablePda, winnerWallet })
    .rpc();
};

/** Reveal hole-card mask at showdown. Triggers hand evaluation and pot settlement on-chain. */
export const revealShowdown = async (
  wallet: WalletContextState,
  idl: object,
  tablePda: PublicKey,
  mask: bigint,
  playerAWallet: PublicKey,
  playerBWallet: PublicKey,
): Promise<string> => {
  const provider = makeProvider(wallet);
  const program = new Program(idl as any, provider);

  return (program.methods as any)
    .revealShowdown(new BN(mask.toString()))
    .accounts({ player: wallet.publicKey!, table: tablePda, playerAWallet, playerBWallet })
    .rpc();
};
