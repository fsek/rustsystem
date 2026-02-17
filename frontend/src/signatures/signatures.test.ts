/**
 * Tests for the BBS+ blind signature commitment implementation.
 *
 * These tests verify:
 * 1. Determinism of generator creation (same inputs → same G1 points)
 * 2. Structural validity of the commitment JSON
 * 3. Correctness of helper functions
 * 4. Token generation produces the expected shapes and sizes
 *
 * Wire-format compatibility with the Rust server is the highest-risk part and
 * must be verified via integration tests (cargo test) once the client is wired up.
 */

import { describe, it, expect } from "vitest";
import {
  createBlindGenerators,
  hashToScalar,
  messageToScalar,
  commit,
  buildProofContext,
  generateToken,
  buildBallot,
  uuidToBytes,
} from "./signatures";

// ─── Helpers ─────────────────────────────────────────────────────────────────

function isHex(s: unknown, expectedBytes: number): boolean {
  return (
    typeof s === "string" &&
    s.length === expectedBytes * 2 &&
    /^[0-9a-f]+$/.test(s)
  );
}

const FIXED_UUID = "550e8400-e29b-41d4-a716-446655440000";
const FIXED_UUID2 = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

// ─── uuidToBytes ─────────────────────────────────────────────────────────────

describe("uuidToBytes", () => {
  it("returns 16 bytes", () => {
    expect(uuidToBytes(FIXED_UUID).length).toBe(16);
  });

  it("parses correctly", () => {
    const bytes = uuidToBytes("00010203-0405-0607-0809-0a0b0c0d0e0f");
    for (let i = 0; i < 16; i++) {
      expect(bytes[i]).toBe(i);
    }
  });

  it("is deterministic", () => {
    const a = uuidToBytes(FIXED_UUID);
    const b = uuidToBytes(FIXED_UUID);
    expect(a).toEqual(b);
  });
});

// ─── createBlindGenerators ────────────────────────────────────────────────────

describe("createBlindGenerators", () => {
  it("returns the requested count of generators", () => {
    const gens = createBlindGenerators(2);
    expect(gens.length).toBe(2);
  });

  it("each generator serialises to 48 compressed bytes", () => {
    const gens = createBlindGenerators(2);
    for (const g of gens) {
      expect(g.toBytes().length).toBe(48);
    }
  });

  it("generators are deterministic (same output for same inputs)", () => {
    const gens1 = createBlindGenerators(2);
    const gens2 = createBlindGenerators(2);
    expect(gens1[0].toBytes()).toEqual(gens2[0].toBytes());
    expect(gens1[1].toBytes()).toEqual(gens2[1].toBytes());
  });

  it("Q2 and J0 are distinct points", () => {
    const [Q2, J0] = createBlindGenerators(2);
    expect(Q2.toBytes()).not.toEqual(J0.toBytes());
  });

  it("first two generators are consistent when requesting more", () => {
    const gens2 = createBlindGenerators(2);
    const gens4 = createBlindGenerators(4);
    expect(gens4[0].toBytes()).toEqual(gens2[0].toBytes());
    expect(gens4[1].toBytes()).toEqual(gens2[1].toBytes());
  });
});

// ─── hashToScalar / messageToScalar ──────────────────────────────────────────

describe("hashToScalar", () => {
  const dst = new TextEncoder().encode("test-dst");

  it("returns a bigint", () => {
    const s = hashToScalar(new Uint8Array([1, 2, 3]), dst);
    expect(typeof s).toBe("bigint");
  });

  it("is deterministic", () => {
    const msg = new Uint8Array([4, 5, 6]);
    const s1 = hashToScalar(msg, dst);
    const s2 = hashToScalar(msg, dst);
    expect(s1).toBe(s2);
  });

  it("is in the scalar field (0 ≤ s < r)", () => {
    const r =
      0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001n;
    const s = hashToScalar(new Uint8Array(48), dst);
    expect(s >= 0n).toBe(true);
    expect(s < r).toBe(true);
  });

  it("different messages produce different scalars", () => {
    const s1 = hashToScalar(new Uint8Array([1]), dst);
    const s2 = hashToScalar(new Uint8Array([2]), dst);
    expect(s1).not.toBe(s2);
  });
});

describe("messageToScalar", () => {
  it("is deterministic", () => {
    const token = new Uint8Array(32).fill(0xab);
    expect(messageToScalar(token)).toBe(messageToScalar(token));
  });

  it("uses a different DST than a plain hashToScalar call", () => {
    const msg = new Uint8Array(32).fill(1);
    const dst = new TextEncoder().encode("different-dst");
    expect(messageToScalar(msg)).not.toBe(hashToScalar(msg, dst));
  });
});

// ─── commit ───────────────────────────────────────────────────────────────────

describe("commit", () => {
  const token = new Uint8Array(256).fill(0x42);

  it("returns commitmentJson and blindFactor", () => {
    const result = commit(token);
    expect(result.commitmentJson).toBeDefined();
    expect(result.blindFactor).toBeDefined();
  });

  it("blindFactor is 32 bytes", () => {
    const { blindFactor } = commit(token);
    expect(blindFactor.length).toBe(32);
  });

  it("commitmentJson has the BBSplus wrapper key", () => {
    const { commitmentJson } = commit(token);
    expect(typeof commitmentJson.BBSplus).toBe("object");
  });

  it("commitment field is a 48-byte hex string", () => {
    const { commitmentJson } = commit(token);
    expect(isHex(commitmentJson.BBSplus.commitment, 48)).toBe(true);
  });

  it("proof.s_cap is a 32-byte hex string", () => {
    const { commitmentJson } = commit(token);
    expect(isHex(commitmentJson.BBSplus.proof.s_cap, 32)).toBe(true);
  });

  it("proof.m_cap is an array of one 32-byte hex string", () => {
    const { commitmentJson } = commit(token);
    const mCap = commitmentJson.BBSplus.proof.m_cap;
    expect(Array.isArray(mCap)).toBe(true);
    expect(mCap.length).toBe(1);
    expect(isHex(mCap[0], 32)).toBe(true);
  });

  it("proof.challenge is a 32-byte hex string", () => {
    const { commitmentJson } = commit(token);
    expect(isHex(commitmentJson.BBSplus.proof.challenge, 32)).toBe(true);
  });

  it("different tokens produce different commitments", () => {
    const t1 = new Uint8Array(256).fill(0x01);
    const t2 = new Uint8Array(256).fill(0x02);
    const c1 = commit(t1).commitmentJson.BBSplus.commitment;
    const c2 = commit(t2).commitmentJson.BBSplus.commitment;
    expect(c1).not.toBe(c2);
  });

  it("same token produces different commitments each call (random blind factor)", () => {
    const c1 = commit(token).commitmentJson.BBSplus.commitment;
    const c2 = commit(token).commitmentJson.BBSplus.commitment;
    // Extremely unlikely to be equal due to random secret_prover_blind
    expect(c1).not.toBe(c2);
  });

  it("JSON serialises cleanly (can be sent as request body)", () => {
    const { commitmentJson } = commit(token);
    expect(() => JSON.stringify(commitmentJson)).not.toThrow();
  });
});

// ─── buildProofContext ────────────────────────────────────────────────────────

describe("buildProofContext", () => {
  const voterBytes = uuidToBytes(FIXED_UUID);
  const meetingBytes = uuidToBytes(FIXED_UUID2);

  it("includes voter_id as number array of length 16", () => {
    const ctx = buildProofContext(voterBytes, meetingBytes);
    expect(Array.isArray(ctx.voter_id)).toBe(true);
    expect(ctx.voter_id.length).toBe(16);
  });

  it("includes meeting_id as number array of length 16", () => {
    const ctx = buildProofContext(voterBytes, meetingBytes);
    expect(ctx.meeting_id.length).toBe(16);
  });

  it("includes registration_timestamp as 8-byte number array", () => {
    const ctx = buildProofContext(voterBytes, meetingBytes);
    expect(ctx.registration_timestamp.length).toBe(8);
  });

  it("includes checksum as 32 zeros", () => {
    const ctx = buildProofContext(voterBytes, meetingBytes);
    expect(ctx.checksum.length).toBe(32);
    expect(ctx.checksum.every((b) => b === 0)).toBe(true);
  });

  it("all fields are arrays of numbers in [0, 255]", () => {
    const ctx = buildProofContext(voterBytes, meetingBytes);
    for (const field of [
      ctx.voter_id,
      ctx.meeting_id,
      ctx.registration_timestamp,
      ctx.checksum,
    ]) {
      expect(field.every((b) => b >= 0 && b <= 255)).toBe(true);
    }
  });
});

// ─── generateToken ────────────────────────────────────────────────────────────

describe("generateToken", () => {
  const voterBytes = uuidToBytes(FIXED_UUID);
  const meetingBytes = uuidToBytes(FIXED_UUID2);

  it("returns token of 256 bytes", () => {
    const result = generateToken(voterBytes, meetingBytes);
    expect(result.token.length).toBe(256);
  });

  it("returns blindFactor of 32 bytes", () => {
    const result = generateToken(voterBytes, meetingBytes);
    expect(result.blindFactor.length).toBe(32);
  });

  it("returns a valid commitmentJson", () => {
    const result = generateToken(voterBytes, meetingBytes);
    expect(isHex(result.commitmentJson.BBSplus.commitment, 48)).toBe(true);
  });

  it("returns a valid context", () => {
    const result = generateToken(voterBytes, meetingBytes);
    expect(result.context.voter_id.length).toBe(16);
    expect(result.context.checksum.length).toBe(32);
  });

  it("two calls produce different tokens", () => {
    const r1 = generateToken(voterBytes, meetingBytes);
    const r2 = generateToken(voterBytes, meetingBytes);
    expect(r1.token).not.toEqual(r2.token);
  });
});

// ─── buildBallot ──────────────────────────────────────────────────────────────

describe("buildBallot", () => {
  const metadata = {
    candidates: ["Alice", "Bob"],
    max_choices: 1,
    protocol_version: 1,
  };
  const token = new Uint8Array(256).fill(0x55);
  const blindFactor = new Uint8Array(32).fill(0xaa);
  const fakeSignature = { BBSplus: { a: "deadbeef" } };

  it("produces an object with metadata, choice, validation, _padding", () => {
    const ballot = buildBallot(
      metadata,
      null,
      token,
      blindFactor,
      fakeSignature,
    );
    expect((ballot as Record<string, unknown>).metadata).toBeDefined();
    expect((ballot as Record<string, unknown>).validation).toBeDefined();
    expect(Object.prototype.hasOwnProperty.call(ballot, "_padding")).toBe(true);
  });

  it("serialised JSON is at least 1024 bytes", () => {
    const ballot = buildBallot(
      metadata,
      [0],
      token,
      blindFactor,
      fakeSignature,
    );
    const json = JSON.stringify(ballot);
    expect(json.length).toBeGreaterThanOrEqual(1024);
  });

  it("validation.proof is an array of 32 numbers", () => {
    const ballot = buildBallot(
      metadata,
      null,
      token,
      blindFactor,
      fakeSignature,
    ) as Record<string, unknown>;
    const v = ballot.validation as Record<string, unknown>;
    expect(Array.isArray(v.proof)).toBe(true);
    expect((v.proof as unknown[]).length).toBe(32);
  });

  it("validation.token is an array of 256 numbers", () => {
    const ballot = buildBallot(
      metadata,
      null,
      token,
      blindFactor,
      fakeSignature,
    ) as Record<string, unknown>;
    const v = ballot.validation as Record<string, unknown>;
    expect((v.token as unknown[]).length).toBe(256);
  });

  it("choice null is preserved", () => {
    const ballot = buildBallot(
      metadata,
      null,
      token,
      blindFactor,
      fakeSignature,
    ) as Record<string, unknown>;
    expect(ballot.choice).toBeNull();
  });

  it("choice [0] is preserved", () => {
    const ballot = buildBallot(
      metadata,
      [0],
      token,
      blindFactor,
      fakeSignature,
    ) as Record<string, unknown>;
    expect(ballot.choice).toEqual([0]);
  });
});
