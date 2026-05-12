import { PublicKey } from "@solana/web3.js";

const env = (key: string) => (import.meta.env as Record<string, string>)[key]?.trim() ?? "";

const PROGRAM_ID_RAW      = env("VITE_PROGRAM_ID")              || "11111111111111111111111111111111";
const ARCIUM_PROGRAM_RAW  = env("VITE_ARCIUM_PROGRAM_ID")       || "11111111111111111111111111111111";
const MXE_RAW             = env("VITE_ARCIUM_MXE_PUBKEY")       || "11111111111111111111111111111111";
const MEMPOOL_RAW         = env("VITE_ARCIUM_MEMPOOL_PUBKEY")   || "11111111111111111111111111111111";
const EXECPOOL_RAW        = env("VITE_ARCIUM_EXECPOOL_PUBKEY")  || "11111111111111111111111111111111";
const CLUSTER_RAW         = env("VITE_ARCIUM_CLUSTER_PUBKEY")   || "11111111111111111111111111111111";
const COMP_DEF_RAW        = env("VITE_ARCIUM_COMP_DEF_PUBKEY")  || "11111111111111111111111111111111";
const FEE_POOL_RAW        = env("VITE_ARCIUM_FEE_POOL_PUBKEY")  || "11111111111111111111111111111111";
const CLOCK_RAW           = env("VITE_ARCIUM_CLOCK_PUBKEY")     || "11111111111111111111111111111111";

export const programId            = new PublicKey(PROGRAM_ID_RAW);
export const arciumProgramId      = new PublicKey(ARCIUM_PROGRAM_RAW);
export const arciumMxeAccount     = new PublicKey(MXE_RAW);
export const arciumMempoolAccount = new PublicKey(MEMPOOL_RAW);
export const arciumExecpoolAccount= new PublicKey(EXECPOOL_RAW);
export const arciumClusterAccount = new PublicKey(CLUSTER_RAW);
export const arciumCompDefAccount = new PublicKey(COMP_DEF_RAW);
export const arciumFeePoolAccount = new PublicKey(FEE_POOL_RAW);
export const arciumClockAccount   = new PublicKey(CLOCK_RAW);

export const TABLE_SEED = Buffer.from("holdem");
export const SIGN_PDA_SEED = Buffer.from("arcium_sign_pda");

export const deriveTablePda = (playerA: PublicKey): PublicKey =>
  PublicKey.findProgramAddressSync([TABLE_SEED, playerA.toBytes()], programId)[0];

export const deriveSignPda = (): PublicKey =>
  PublicKey.findProgramAddressSync([SIGN_PDA_SEED], programId)[0];

export const deriveComputationAccount = (computationOffset: bigint): PublicKey => {
  const offsetBuf = new ArrayBuffer(8);
  new DataView(offsetBuf).setBigUint64(0, computationOffset, true);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("computation"), arciumMxeAccount.toBytes(), new Uint8Array(offsetBuf)],
    arciumProgramId,
  )[0];
};

export const isConfigured = () =>
  MXE_RAW !== "11111111111111111111111111111111" && MEMPOOL_RAW !== "11111111111111111111111111111111";
