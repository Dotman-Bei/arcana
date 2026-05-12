/**
 * Client-side Enc<Shared> encryption for the deal_cards MXE circuit.
 *
 * The DeckInput struct has three u64 fields: seed, p1_mask, p2_mask.
 * All three are encrypted together under one ephemeral X25519 keypair
 * (single Enc<Shared, DeckInput>), matching how ArgBuilder expects them.
 *
 * Scheme (same as SILENT CIRCLE arcium-encrypt.ts):
 *   1. Generate ephemeral X25519 keypair.
 *   2. X25519 DH with MXE cluster pubkey → 32-byte shared secret.
 *   3. HKDF-SHA-256(secret, info="arcium-enc-shared-v1") → AES-256 key.
 *   4. Encrypt each u64 field: zero-pad to 16 bytes, AES-256-GCM with
 *      per-field nonce = nonceBase[0..8] || fieldIndex_u32_LE → 32-byte ciphertext.
 */

export type EncryptedDeckInput = {
  pubkey: Uint8Array;        // ephemeral X25519 public key [32]
  nonce: bigint;             // u128 nonce passed as plaintext_u128
  seedEnc: Uint8Array;       // AES-256-GCM ciphertext of seed [32]
  p1MaskEnc: Uint8Array;     // AES-256-GCM ciphertext of p1_mask [32]
  p2MaskEnc: Uint8Array;     // AES-256-GCM ciphertext of p2_mask [32]
};

const encodeU32Le = (v: number): Uint8Array => {
  const b = new ArrayBuffer(4);
  new DataView(b).setUint32(0, v, true);
  return new Uint8Array(b);
};

const encodeU64Le = (v: bigint): Uint8Array => {
  const b = new ArrayBuffer(8);
  new DataView(b).setBigUint64(0, v, true);
  return new Uint8Array(b);
};

const deriveAesKey = async (sharedSecretBuf: ArrayBuffer): Promise<CryptoKey> => {
  const keyMat = await crypto.subtle.importKey("raw", sharedSecretBuf, "HKDF", false, ["deriveKey"]);
  return crypto.subtle.deriveKey(
    {
      name: "HKDF",
      hash: "SHA-256",
      salt: new Uint8Array(32),
      info: new TextEncoder().encode("arcium-enc-shared-v1"),
    },
    keyMat,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt"],
  );
};

const encryptField = async (
  aesKey: CryptoKey,
  nonceBase: Uint8Array,
  fieldIndex: number,
  value8: Uint8Array,
): Promise<Uint8Array> => {
  const iv = new Uint8Array(12);
  iv.set(nonceBase.slice(0, 8), 0);
  iv.set(encodeU32Le(fieldIndex), 8);
  const plain16 = new Uint8Array(16);
  plain16.set(value8.slice(0, 8));
  return new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, aesKey, plain16));
};

/**
 * Encrypt the three DeckInput fields (seed, p1_mask, p2_mask) for the MXE.
 *
 * @param seed     Combined deck entropy: salt_a XOR salt_b (computed client-side)
 * @param p1Mask   Player A's secret XOR mask (64-bit random)
 * @param p2Mask   Player B's secret XOR mask (64-bit random) — provided by B
 * @param clusterX25519  Raw 32-byte MXE cluster X25519 public key
 */
export const encryptDeckInput = async (
  seed: bigint,
  p1Mask: bigint,
  p2Mask: bigint,
  clusterX25519: Uint8Array,
): Promise<EncryptedDeckInput> => {
  const ephKP = await crypto.subtle.generateKey({ name: "X25519" } as EcKeyGenParams, true, ["deriveBits"]) as CryptoKeyPair;
  const rawEphPub = new Uint8Array(await crypto.subtle.exportKey("raw", ephKP.publicKey));

  const clusterPub = await crypto.subtle.importKey("raw", clusterX25519, { name: "X25519" } as EcKeyImportParams, false, []);
  const sharedBuf = await crypto.subtle.deriveBits({ name: "X25519", public: clusterPub } as EcdhKeyDeriveParams, ephKP.privateKey, 256);
  const aesKey = await deriveAesKey(sharedBuf);

  const nonceBytes = new Uint8Array(16);
  crypto.getRandomValues(nonceBytes);
  const view = new DataView(nonceBytes.buffer);
  const nonceLo = view.getBigUint64(0, true);
  const nonceHi = view.getBigUint64(8, true);
  const nonce = nonceLo | (nonceHi << 64n);
  const nonceBase = nonceBytes.slice(0, 8);

  const seedEnc    = await encryptField(aesKey, nonceBase, 0, encodeU64Le(seed));
  const p1MaskEnc  = await encryptField(aesKey, nonceBase, 1, encodeU64Le(p1Mask));
  const p2MaskEnc  = await encryptField(aesKey, nonceBase, 2, encodeU64Le(p2Mask));

  return { pubkey: rawEphPub, nonce, seedEnc, p1MaskEnc, p2MaskEnc };
};

/** Generate a cryptographically random 64-bit mask for a player's hole cards. */
export const generatePlayerMask = (): bigint => {
  const buf = new Uint8Array(8);
  crypto.getRandomValues(buf);
  return new DataView(buf.buffer).getBigUint64(0, true);
};

/** Keccak-256-equivalent commitment: SHA-256(mask_le || table_pubkey_bytes). */
export const computeMaskCommit = async (
  mask: bigint,
  tablePubkeyBytes: Uint8Array,
): Promise<Uint8Array> => {
  const preimage = new Uint8Array(40);
  const maskBytes = new Uint8Array(8);
  new DataView(maskBytes.buffer).setBigUint64(0, mask, true);
  preimage.set(maskBytes, 0);
  preimage.set(tablePubkeyBytes, 8);
  return new Uint8Array(await crypto.subtle.digest("SHA-256", preimage));
};
