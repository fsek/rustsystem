/**
 * Runs Argon2id off the main thread. It takes seconds and noble only yields to microtasks, so
 * on the main thread the page would not repaint (no spinner) until it finished.
 * Used by `cryptoGen.ts`; the password stays in this worker's memory only for the call.
 */

import { argon2id } from "@noble/hashes/argon2.js";

export interface Argon2Request {
  password: string;
  salt: Uint8Array;
  t: number;
  m: number;
  p: number;
  dkLen: number;
}

export type Argon2Response =
  | { ok: true; key: Uint8Array }
  | { ok: false; error: string };

self.onmessage = (e: MessageEvent<Argon2Request>) => {
  const { password, salt, ...opts } = e.data;
  let res: Argon2Response;
  try {
    res = { ok: true, key: argon2id(password, salt, opts) };
  } catch (err) {
    res = {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
  self.postMessage(res);
};
