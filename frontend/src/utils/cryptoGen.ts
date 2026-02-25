// derivePublicKey.ts
//
// Browser/Vite-friendly TypeScript: password + saltHex -> deterministic public key.
// Uses WebCrypto (PBKDF2-HMAC-SHA256) + libsodium (Ed25519 keypair from seed).
//
// Install:
//   npm i libsodium-wrappers
//
// Notes:
// - This matches the OpenSSL-style PBKDF2-HMAC-SHA256 derivation:
//   seed = PBKDF2(password_utf8, salt_bytes, iterations, 32, SHA-256)
// - Then:
//   (pk, sk) = Ed25519_keypair_from_seed(seed)
// - Public key output is 32 bytes.
//
// If your goal is "server encrypts with public key", you probably want X25519 instead;
// see the commented alternative at the bottom.

import sodium from "libsodium-wrappers";

function bytesToBase64(bytes: Uint8Array): string {
  let bin = "";
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin);
}

function wrapPem(b64: string, label: string): string {
  const lines = b64.match(/.{1,64}/g) ?? [];
  return `-----BEGIN ${label}-----\n${lines.join("\n")}\n-----END ${label}-----\n`;
}

function concat(...parts: Uint8Array[]): Uint8Array {
  const len = parts.reduce((a, p) => a + p.length, 0);
  const out = new Uint8Array(len);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

export function x25519PublicKeyToPem(rawPub: Uint8Array): string {
  if (rawPub.length !== 32)
    throw new Error("X25519 public key must be 32 bytes");

  const spkiPrefix = new Uint8Array([
    0x30, 0x2a, 0x30, 0x05,
    // OID 1.3.101.110 (X25519) = 2B 65 6E
    0x06, 0x03, 0x2b, 0x65, 0x6e, 0x03, 0x21, 0x00,
  ]);

  const der = concat(spkiPrefix, rawPub);
  const b64 = bytesToBase64(der);
  return wrapPem(b64, "PUBLIC KEY");
}

/** Convert even-length hex string to bytes. */
function hexToBytes(hex: string): Uint8Array {
  if (!/^[0-9a-fA-F]+$/.test(hex) || hex.length % 2 !== 0) {
    throw new Error("saltHex must be even-length hex");
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/** Convert bytes to hex (useful for logging/comparison). */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Derive a 32-byte seed using PBKDF2-HMAC-SHA256 in the browser.
 */
async function deriveSeedPBKDF2_SHA256(
  password: string,
  saltBytes: Uint8Array,
  iterations: number,
): Promise<Uint8Array> {
  if (iterations <= 0) throw new Error("iterations must be > 0");

  const passwordBytes = new TextEncoder().encode(password);

  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    passwordBytes,
    { name: "PBKDF2" },
    false,
    ["deriveBits"],
  );

  const bits = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      hash: "SHA-256",
      salt: saltBytes,
      iterations,
    },
    keyMaterial,
    32 * 8, // 32 bytes
  );

  return new Uint8Array(bits);
}

/**
 * Returns the raw 32-byte X25519 private key (= PBKDF2 seed) derived from a
 * password. This is the same scalar that crypto_scalarmult_base maps to the
 * public key stored on the server, so it can be used to ECDH-decrypt tally
 * files encrypted for that public key.
 */
export async function deriveX25519PrivateKeyFromPassword(params: {
  password: string;
  saltHex: string;
  iterations: number;
}): Promise<Uint8Array> {
  const saltBytes = hexToBytes(params.saltHex);
  return deriveSeedPBKDF2_SHA256(params.password, saltBytes, params.iterations);
}

/**
 * FULL PIPELINE:
 * password + saltHex -> PBKDF2 seed -> deterministic Ed25519 public key
 */
export async function deriveEd25519PublicKeyFromPassword(params: {
  password: string;
  saltHex: string;
  iterations: number;
}): Promise<Uint8Array> {
  await sodium.ready;

  const saltBytes = hexToBytes(params.saltHex);
  const seed = await deriveSeedPBKDF2_SHA256(
    params.password,
    saltBytes,
    params.iterations,
  );

  // Derive X25519 public key directly from seed as private scalar.
  // crypto_scalarmult_base uses the seed directly (with internal clamping),
  // matching OpenSSL's behaviour when the raw bytes are written into the DER.
  return sodium.crypto_scalarmult_base(seed); // 32 bytes
}

/*
========================================
If you actually need an ENCRYPTION public key (server encrypts, client decrypts):
Use X25519 (crypto_box) instead of Ed25519 (crypto_sign).

Replace:
  sodium.crypto_sign_seed_keypair(seed)
with:
  sodium.crypto_box_seed_keypair(seed)

Then:
  return kp.publicKey

Example:
  const kp = sodium.crypto_box_seed_keypair(seed);
========================================
*/
