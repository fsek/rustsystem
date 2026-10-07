// @vitest-environment node
import { describe, expect, it } from "vitest";

import { hexToBytes } from "@/api/encoding";
import { decryptTallyFiles, parseHeader } from "./tallyDecrypt";

/**
 * A tally file written by the real server code (`rustsystem_server::tally::encrypt`) for the
 * password "fixture password", with cheap Argon2id costs (t=1, m=8 MiB) so the test is fast.
 * If the browser can't decrypt this, hosts can't read their results.
 */
const FIXTURE = hexToBytes(
  "5253544c590201000102030405060708090a0b0c0d0e0f0000000100002000011f8a46ae31aa5898dfc03299684744d2f2b0b91232a91b52c498462756c220352f823c7a43f07b4db6b13768be9cbc148a7bb3b0ecec6f40a4e20b4dad5539d3e77ce85f13f7dcffa3dc2d60092ed99b9cb1cebcc6a00fc21f782abcaa5bd2ff2fbc540dd71274c4df6c83d52053dc19eb55452710bf36f06806122fff38e9b1fe4c863220f54eb6ac043aacb324c6e7d1f0c5625dd46510807d33685dda2e14534e175ada0476d3a9323296b432692ba50b5585dd629fd614224bcb185615f2d27596de7b0e607ca9631274c15fe9d22f2c81f11303133785fbcd77b96bf879ed423550a0c9c5f5b86042a6a18692d1c141f040b212c6df2bc4e894e4522091efbc8712afed64d8c6bffd8a39b8e27f4e6a222892432f60fead48a39e5745fd65480e764a6f89ca7dca336ae14b3dbf0f14ab5b6422d05590e0c44f0665b9dd1af0859871ef3b4effb72cf176b96b5208401c2b81dba45421f20e1e520be4c1aa9a7de087e766",
);

describe("tally files", () => {
  it("reads the KDF parameters from the header", () => {
    expect(parseHeader(FIXTURE)).toEqual({
      salt_hex: "000102030405060708090a0b0c0d0e0f",
      t_cost: 1,
      m_cost_kib: 8192,
      p_cost: 1,
    });
  });

  it("decrypts a file written by the Rust server", async () => {
    const [tally] = await decryptTallyFiles([FIXTURE], "fixture password");
    expect(tally.meeting).toBe("Fixture");
    expect(tally.candidates).toEqual(["Anna", "Bo"]);
    expect(tally.score).toEqual([12, 9]);
    expect(tally.blank).toBe(2);
    expect(tally.counts).toEqual({ eligible: 25, signed: 23, received: 23 });
  }, 30_000);

  it("rejects the wrong password", async () => {
    await expect(decryptTallyFiles([FIXTURE], "wrong")).rejects.toThrow();
  }, 30_000);

  it("rejects a tampered header", async () => {
    const tampered = FIXTURE.slice();
    tampered[8] ^= 1; // a salt byte: authenticated, so decryption must fail
    await expect(
      decryptTallyFiles([tampered], "fixture password"),
    ).rejects.toThrow();
  }, 30_000);

  it("rejects other files", () => {
    expect(() => parseHeader(new Uint8Array(100))).toThrow(/not a Rustsystem/);
    expect(() => parseHeader(new Uint8Array(10))).toThrow(/too short/);
  });
});
