// @vitest-environment node
import { describe, expect, it } from "vitest";

import { x25519 } from "@noble/curves/ed25519.js";

import { fromBase64Url } from "@/api/encoding";
import {
  KDF_DEFAULTS,
  SALT_LEN,
  bytesToHex,
  deriveX25519PrivateKeyFromPassword,
  hexToBytes,
  newKdfParams,
  newTallyKey,
} from "./cryptoGen";

describe("newKdfParams", () => {
  it("produces a 16-byte salt and the default costs", () => {
    const kdf = newKdfParams();
    expect(hexToBytes(kdf.salt_hex)).toHaveLength(SALT_LEN);
    expect(kdf.t_cost).toBe(KDF_DEFAULTS.t_cost);
    expect(kdf.m_cost_kib).toBe(KDF_DEFAULTS.m_cost_kib);
    expect(kdf.p_cost).toBe(KDF_DEFAULTS.p_cost);
  });

  it("generates a different salt every time", () => {
    // The whole point of the change: one global salt meant a single precomputation
    // attack broke every meeting at once.
    const salts = new Set(
      Array.from({ length: 16 }, () => newKdfParams().salt_hex),
    );
    expect(salts.size).toBe(16);
  });

  it("stays inside the bounds the server accepts", () => {
    // Mirrors Kdf::new in rustsystem-server/src/tally.rs.
    const kdf = newKdfParams();
    expect(kdf.t_cost).toBeGreaterThanOrEqual(1);
    expect(kdf.t_cost).toBeLessThanOrEqual(10);
    expect(kdf.m_cost_kib).toBeGreaterThanOrEqual(8 * 1024);
    expect(kdf.m_cost_kib).toBeLessThanOrEqual(1024 * 1024);
    expect(kdf.p_cost).toBeGreaterThanOrEqual(1);
    expect(kdf.p_cost).toBeLessThanOrEqual(4);
  });
});

describe("Argon2id derivation", () => {
  /**
   * Cross-implementation test vector.
   *
   * The browser derives the meeting keypair with @noble/hashes; `decrypt-tally` derives
   * it with the Rust `argon2` crate. If the two ever disagree, a host silently loses the
   * ability to decrypt their own tallies. The mirror of this test lives in
   * `decrypt-tally/src/main.rs`.
   */
  it("matches the Rust argon2 crate on a shared vector", async () => {
    const key = await deriveX25519PrivateKeyFromPassword({
      password: "correct horse battery staple",
      kdf: {
        salt_hex: "00112233445566778899aabbccddeeff",
        t_cost: 3,
        m_cost_kib: 65536,
        p_cost: 1,
      },
    });

    expect(bytesToHex(key)).toBe(
      "c63a7e80f29a251ff0f1067c51d08ff12594199c5d2bd4a51d95348f3a205883",
    );

    const key2 = await deriveX25519PrivateKeyFromPassword({
      password: "correct horse battery staple",
      kdf: {
        salt_hex: "07".repeat(16),
        t_cost: 3,
        m_cost_kib: 65536,
        p_cost: 1,
      },
    });
    expect(bytesToHex(key2)).toBe(
      "6ad10af97f1744119bd7135c85121dc589794f9c5d646200b8ad4d6becf15084",
    );
  }, 30_000);

  it("derives a different key for a different salt", async () => {
    const cheap = { t_cost: 1, m_cost_kib: 8 * 1024, p_cost: 1 };
    const a = await deriveX25519PrivateKeyFromPassword({
      password: "same password",
      kdf: { salt_hex: "0".repeat(32), ...cheap },
    });
    const b = await deriveX25519PrivateKeyFromPassword({
      password: "same password",
      kdf: { salt_hex: "f".repeat(32), ...cheap },
    });

    expect(bytesToHex(a)).not.toBe(bytesToHex(b));
  }, 30_000);

  it("rejects a salt that is not 16 bytes", async () => {
    await expect(
      deriveX25519PrivateKeyFromPassword({
        password: "pw",
        kdf: { salt_hex: "abcd", t_cost: 1, m_cost_kib: 8192, p_cost: 1 },
      }),
    ).rejects.toThrow();
  });
});

describe("newTallyKey", () => {
  it("sends only the public key, a fresh salt and the costs", async () => {
    const body = await newTallyKey("pw");
    expect(fromBase64Url(body.publicKey)).toHaveLength(32);
    expect(fromBase64Url(body.salt)).toHaveLength(SALT_LEN);
    expect(body.tCost).toBe(KDF_DEFAULTS.t_cost);
    expect(body.mCostKib).toBe(KDF_DEFAULTS.m_cost_kib);
    expect(body.pCost).toBe(KDF_DEFAULTS.p_cost);
    expect(JSON.stringify(body)).not.toContain("pw");
  }, 30_000);

  it("is the public key of what the password derives", async () => {
    const body = await newTallyKey("hunter2");
    const priv = await deriveX25519PrivateKeyFromPassword({
      password: "hunter2",
      kdf: {
        salt_hex: bytesToHex(fromBase64Url(body.salt)),
        t_cost: body.tCost,
        m_cost_kib: body.mCostKib,
        p_cost: body.pCost,
      },
    });
    expect(bytesToHex(x25519.getPublicKey(priv))).toBe(
      bytesToHex(fromBase64Url(body.publicKey)),
    );
  }, 30_000);
});
