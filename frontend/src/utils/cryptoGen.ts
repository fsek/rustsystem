/**
 * The meeting's tally key, derived from the meeting password (`docs/PROTOCOL.md` §4.1, §8).
 *
 *   salt  = 16 random bytes, new for every meeting
 *   seed  = Argon2id(password, salt, t=3, m=64 MiB, p=1)   — 32 bytes
 *   key   = X25519 key pair with `seed` as the private key
 *
 * At meeting creation only the public key, salt and costs go to the server. The salt and costs
 * are written into every tally file, so later the password alone re-derives the private key —
 * here in the browser, or in the `decrypt-tally` CLI, which must produce identical bytes
 * (see the shared test vectors in `cryptoGen.test.ts` and `decrypt-tally/src/main.rs`).
 */

import { x25519 } from "@noble/curves/ed25519.js";
import { argon2idAsync } from "@noble/hashes/argon2.js";

import { bytesToHex, hexToBytes, toBase64Url } from "@/api/encoding";
import type { TallyKeyBody } from "@/api/meeting";
import type { Argon2Request, Argon2Response } from "./argon2.worker";

export { bytesToHex, hexToBytes };

export const SALT_LEN = 16;

/** About a second in a desktop browser, a few on a phone. Only paid at creation and decryption. */
export const KDF_DEFAULTS = {
  t_cost: 3,
  m_cost_kib: 64 * 1024,
  p_cost: 1,
} as const;

export interface KdfParams {
  salt_hex: string;
  t_cost: number;
  m_cost_kib: number;
  p_cost: number;
}

/** Fresh parameters for a new meeting: the default costs and a new random salt. */
export function newKdfParams(): KdfParams {
  const salt = crypto.getRandomValues(new Uint8Array(SALT_LEN));
  return { salt_hex: bytesToHex(salt), ...KDF_DEFAULTS };
}

/** The X25519 private key for `password` under `kdf`. Yields to the UI while it works. */
export async function deriveX25519PrivateKeyFromPassword(params: {
  password: string;
  kdf: KdfParams;
}): Promise<Uint8Array> {
  const salt = hexToBytes(params.kdf.salt_hex);
  if (salt.length !== SALT_LEN)
    throw new Error(`The salt must be ${SALT_LEN} bytes.`);
  const opts = {
    t: params.kdf.t_cost,
    m: params.kdf.m_cost_kib,
    p: params.kdf.p_cost,
    dkLen: 32,
  };
  // No workers outside the browser (unit tests): run it here instead.
  if (typeof Worker === "undefined")
    return argon2idAsync(params.password, salt, { ...opts, asyncTick: 20 });
  return argon2idInWorker({ password: params.password, salt, ...opts });
}

/** One worker per call: this runs a few times per meeting, and it frees the 64 MiB after. */
function argon2idInWorker(req: Argon2Request): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./argon2.worker.ts", import.meta.url), {
      type: "module",
    });
    worker.onmessage = (e: MessageEvent<Argon2Response>) => {
      worker.terminate();
      if (e.data.ok) resolve(e.data.key);
      else reject(new Error(`Argon2id: ${e.data.error}`));
    };
    worker.onerror = (e) => {
      worker.terminate();
      reject(new Error(`Argon2id worker: ${e.message}`));
    };
    worker.postMessage(req);
  });
}

export async function deriveX25519PublicKeyFromPassword(params: {
  password: string;
  kdf: KdfParams;
}): Promise<Uint8Array> {
  return x25519.getPublicKey(await deriveX25519PrivateKeyFromPassword(params));
}

/** What `POST /api/meetings` needs to know about the tally key for a new meeting. */
export async function newTallyKey(password: string): Promise<TallyKeyBody> {
  const kdf = newKdfParams();
  const publicKey = await deriveX25519PublicKeyFromPassword({ password, kdf });
  return {
    publicKey: toBase64Url(publicKey),
    salt: toBase64Url(hexToBytes(kdf.salt_hex)),
    tCost: kdf.t_cost,
    mCostKib: kdf.m_cost_kib,
    pCost: kdf.p_cost,
  };
}
