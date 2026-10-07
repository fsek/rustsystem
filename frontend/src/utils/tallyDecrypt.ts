/**
 * Decrypting tally files in the browser (`docs/PROTOCOL.md` §8). Mirrors
 * `rustsystem-server/src/tally.rs` and `decrypt-tally/src/main.rs`.
 *
 * Container v2:
 *   magic "RSTLY" (5) | version 2 (1) | kdf id 1 (1) | salt (16) | t_cost u32 BE | m_cost KiB u32 BE | p_cost (1)
 *   | ephemeral X25519 public key (32) | nonce (12) | ChaCha20-Poly1305 ciphertext+tag
 * The first 32 bytes are authenticated as associated data.
 *
 * The salt and costs travel in each file, so the password is all that's needed.
 */

import { chacha20poly1305 } from "@noble/ciphers/chacha.js";
import { x25519 } from "@noble/curves/ed25519.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";

import { bytesToHex } from "@/api/encoding";
import {
  type KdfParams,
  deriveX25519PrivateKeyFromPassword,
} from "./cryptoGen";

const MAGIC = "RSTLY";
const VERSION = 2;
const KDF_ARGON2ID = 1;
const HEADER_LEN = 32;
const MIN_LEN = HEADER_LEN + 32 + 12 + 16;
const HKDF_INFO = new TextEncoder().encode("rustsystem-tally-v2");

/** What a decrypted tally file contains (`TallyFile` in `rustsystem-server/src/tally.rs`). */
export interface TallyFile {
  meeting: string;
  round: string;
  round_id: string;
  opened_at: string;
  closed_at: string;
  candidates: string[];
  score: number[];
  blank: number;
  counts: { eligible: number; signed: number | null; received: number };
  participants: string[];
}

export function parseHeader(file: Uint8Array): KdfParams {
  if (file.length < MIN_LEN)
    throw new Error("The file is too short to be a tally file.");
  if (new TextDecoder().decode(file.slice(0, 5)) !== MAGIC)
    throw new Error("This is not a Rustsystem tally file.");
  if (file[5] !== VERSION)
    throw new Error(`Unsupported tally file version ${file[5]}.`);
  if (file[6] !== KDF_ARGON2ID)
    throw new Error(`Unknown key-derivation id ${file[6]}.`);
  const view = new DataView(file.buffer, file.byteOffset, file.byteLength);
  return {
    salt_hex: bytesToHex(file.slice(7, 23)),
    t_cost: view.getUint32(23),
    m_cost_kib: view.getUint32(27),
    p_cost: file[31],
  };
}

export function decryptWithKey(
  file: Uint8Array,
  privateKey: Uint8Array,
): TallyFile {
  const header = file.slice(0, HEADER_LEN);
  const ephemeral = file.slice(HEADER_LEN, HEADER_LEN + 32);
  const nonce = file.slice(HEADER_LEN + 32, HEADER_LEN + 44);
  const shared = x25519.getSharedSecret(privateKey, ephemeral);
  const key = hkdf(sha256, shared, ephemeral, HKDF_INFO, 44).slice(0, 32);
  const plaintext = chacha20poly1305(key, nonce, header).decrypt(
    file.slice(HEADER_LEN + 44),
  );
  return JSON.parse(new TextDecoder().decode(plaintext));
}

/**
 * Decrypts every file with `password`. Files from the same meeting share a salt, so the slow
 * key derivation runs once per distinct set of parameters, not once per file.
 */
export async function decryptTallyFiles(
  files: Uint8Array[],
  password: string,
): Promise<TallyFile[]> {
  const keys = new Map<string, Uint8Array>();
  const out: TallyFile[] = [];
  for (const file of files) {
    const kdf = parseHeader(file);
    const id = JSON.stringify(kdf);
    let key = keys.get(id);
    if (!key) {
      key = await deriveX25519PrivateKeyFromPassword({ password, kdf });
      keys.set(id, key);
    }
    out.push(decryptWithKey(file, key));
  }
  return out;
}
