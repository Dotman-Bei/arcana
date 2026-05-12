/**
 * Arcium infrastructure account utilities for Arcana Hold'em.
 * Adapted from SILENT CIRCLE — identical MXE/Mempool decode logic.
 */

import { type Connection } from "@solana/web3.js";
import { arciumMxeAccount, arciumMempoolAccount } from "@/lib/arcium-config";
import { solanaConnection } from "@/lib/solana";

type ConnectionLike = Pick<Connection, "getAccountInfo">;

// ── MXEAccount Borsh decode ───────────────────────────────────────────────────
const parseMxeX25519 = (raw: Uint8Array): Uint8Array | null => {
  let pos = 8;
  if (pos >= raw.length) return null;
  const hasCluster = raw[pos] === 1; pos += 1;
  if (hasCluster) pos += 4;
  pos += 8; // keygen_offset
  pos += 8; // key_recovery_init_offset
  pos += 32; // mxe_program_id
  if (pos >= raw.length) return null;
  const hasAuthority = raw[pos] === 1; pos += 1;
  if (hasAuthority) pos += 32;
  if (pos >= raw.length) return null;
  const isSet = raw[pos] === 1; pos += 1;
  if (!isSet) return null;
  if (pos + 32 > raw.length) return null;
  const x25519 = raw.slice(pos, pos + 32);
  return x25519.every(b => b === 0) ? null : x25519;
};

export const fetchClusterX25519Pubkey = async (
  connection: ConnectionLike = solanaConnection,
): Promise<Uint8Array | null> => {
  const info = await connection.getAccountInfo(arciumMxeAccount, "confirmed");
  if (!info) return null;
  return parseMxeX25519(new Uint8Array(info.data as Buffer));
};

const envX25519Hex = (() => {
  const raw = (import.meta.env as Record<string, string>)["VITE_ARCIUM_CLUSTER_X25519_PUBKEY_HEX"]?.trim();
  return raw?.length === 64 ? raw : null;
})();

const hexToBytes = (hex: string): Uint8Array => {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  return bytes;
};

export const resolveClusterX25519Pubkey = async (
  connection: ConnectionLike = solanaConnection,
): Promise<Uint8Array | null> => {
  if (envX25519Hex) return hexToBytes(envX25519Hex);
  return fetchClusterX25519Pubkey(connection);
};

// ── MempoolAccount Borsh decode ───────────────────────────────────────────────
const MEMPOOL_COMP_COUNT_OFFSET = 41;

export const fetchNextComputationOffset = async (
  connection: ConnectionLike = solanaConnection,
): Promise<bigint> => {
  try {
    const info = await connection.getAccountInfo(arciumMempoolAccount, "confirmed");
    if (info && info.data.length >= MEMPOOL_COMP_COUNT_OFFSET + 8) {
      const data = info.data as Buffer;
      return new DataView(data.buffer, data.byteOffset, data.byteLength).getBigUint64(
        MEMPOOL_COMP_COUNT_OFFSET,
        true,
      );
    }
  } catch {}
  return BigInt(Date.now());
};
