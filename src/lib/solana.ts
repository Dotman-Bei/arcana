import { Connection, PublicKey } from "@solana/web3.js";

const RPC = (import.meta.env as Record<string, string>)["VITE_SOLANA_RPC_URL"] || "https://api.devnet.solana.com";

export const solanaConnection = new Connection(RPC, "confirmed");

/** Poll for an account to exist (up to maxMs ms). */
export const waitForAccount = async (pubkey: PublicKey, maxMs = 60_000): Promise<boolean> => {
  const deadline = Date.now() + maxMs;
  while (Date.now() < deadline) {
    const info = await solanaConnection.getAccountInfo(pubkey);
    if (info) return true;
    await new Promise(r => setTimeout(r, 2000));
  }
  return false;
};
