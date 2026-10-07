// @vitest-environment node
import { RSABSSA } from "@cloudflare/blindrsa-ts";
import { afterEach, describe, expect, it, vi } from "vitest";

import { fromBase64Url, toBase64Url } from "@/api/encoding";
import { ApiError } from "@/api/error";
import {
  ballotMessage,
  choiceProblem,
  signBallot,
  submitBallot,
} from "./ballot";

const suite = RSABSSA.SHA384.PSS.Randomized();

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Answers `GET /api/config` (where trustauth is) and hands every other request to `sign`. */
function stubTrustauth(sign: (init: RequestInit) => Promise<Response>) {
  vi.stubGlobal("fetch", async (url: string, init: RequestInit) =>
    url.endsWith("/api/config")
      ? jsonResponse(200, { trustauthUrl: "https://trustauth.test" })
      : sign(init),
  );
}

function jsonResponse(status: number, body?: unknown): Response {
  return new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("choiceProblem (mirrors rustsystem-server/src/ballot.rs)", () => {
  it("accepts valid choices and blank", () => {
    expect(choiceProblem([0, 2], 3, 2)).toBeNull();
    expect(choiceProblem([2, 0], 3, 2)).toBeNull(); // order is fixed when the ballot is built
    expect(choiceProblem(null, 3, 1)).toBeNull();
  });

  it("rejects what the server would reject", () => {
    expect(choiceProblem([], 3, 1)).not.toBeNull();
    expect(choiceProblem([0, 0], 3, 2)).not.toBeNull();
    expect(choiceProblem([3], 3, 1)).not.toBeNull();
    expect(choiceProblem([-1], 3, 1)).not.toBeNull();
    expect(choiceProblem([0, 1, 2], 3, 2)).not.toBeNull();
  });
});

describe("ballotMessage", () => {
  it("has the exact shape the server parses, with sorted choices", () => {
    const msg = JSON.parse(
      new TextDecoder().decode(ballotMessage("r-1", [2, 0])),
    );
    expect(Object.keys(msg)).toEqual(["v", "round", "choice", "nonce"]);
    expect(msg.v).toBe(1);
    expect(msg.round).toBe("r-1");
    expect(msg.choice).toEqual([0, 2]);
    expect(fromBase64Url(msg.nonce)).toHaveLength(32);
  });

  it("is unique every time", () => {
    const a = new TextDecoder().decode(ballotMessage("r", null));
    const b = new TextDecoder().decode(ballotMessage("r", null));
    expect(a).not.toBe(b);
  });
});

describe("signBallot", () => {
  async function roundKey() {
    const { privateKey, publicKey } = await suite.generateKey({
      publicExponent: Uint8Array.from([1, 0, 1]),
      modulusLength: 2048,
    });
    const spki = new Uint8Array(
      await crypto.subtle.exportKey("spki", publicKey),
    );
    return { privateKey, publicKey, b64: toBase64Url(spki) };
  }

  it("produces a ballot that verifies, and trustauth only sees a blinded value", async () => {
    const key = await roundKey();
    const seen: string[] = [];
    stubTrustauth(async (init: RequestInit) => {
      const body = JSON.parse(init.body as string);
      seen.push(body.blinded);
      const blindSig = await suite.blindSign(
        key.privateKey,
        fromBase64Url(body.blinded),
      );
      return jsonResponse(200, { blind_sig: toBase64Url(blindSig) });
    });

    const round = {
      id: "r-1",
      candidates: ["A", "B"],
      maxChoices: 1,
      publicKey: key.b64,
    };
    const { prepared, sig } = await signBallot(round, [1]);

    expect(await suite.verify(key.publicKey, sig, prepared)).toBe(true);
    const msg = JSON.parse(new TextDecoder().decode(prepared.slice(32)));
    expect(msg.choice).toEqual([1]);
    expect(seen).toHaveLength(1);
    expect(seen[0]).not.toContain(toBase64Url(prepared.slice(32)));
  }, 30_000);

  it("refuses an invalid choice before asking trustauth", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    const round = {
      id: "r",
      candidates: ["A"],
      maxChoices: 1,
      publicKey: "unused",
    };
    await expect(signBallot(round, [0, 0])).rejects.toBeInstanceOf(ApiError);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("rejects a signature from a key other than the server's (key tagging)", async () => {
    const published = await roundKey();
    const tagged = await roundKey();
    stubTrustauth(async (init: RequestInit) => {
      const blinded = fromBase64Url(JSON.parse(init.body as string).blinded);
      // A trustauth signing with its own, voter-specific key. When the blinded value doesn't
      // fit under that key's modulus, any other answer is just as wrong.
      let blindSig: Uint8Array;
      try {
        blindSig = await suite.blindSign(tagged.privateKey, blinded);
      } catch {
        blindSig = crypto.getRandomValues(new Uint8Array(256));
      }
      return jsonResponse(200, { blind_sig: toBase64Url(blindSig) });
    });
    const round = {
      id: "r",
      candidates: ["A"],
      maxChoices: 1,
      publicKey: published.b64,
    };
    await expect(signBallot(round, [0])).rejects.toMatchObject({
      code: "InvalidSignature",
    });
  }, 30_000);
});

describe("submitBallot", () => {
  const ballot = {
    prepared: new Uint8Array([1, 2, 3]),
    sig: new Uint8Array([4]),
  };
  const noSleep = async () => {};

  it("sends no cookies", async () => {
    const fetchSpy = vi.fn(async () => jsonResponse(204));
    vi.stubGlobal("fetch", fetchSpy);
    await submitBallot("m", ballot, noSleep);
    const init = (
      fetchSpy.mock.calls[0] as unknown as [string, RequestInit]
    )[1];
    expect(init.credentials).toBe("omit");
  });

  it("treats AlreadyReceived as success", async () => {
    vi.stubGlobal("fetch", async () =>
      jsonResponse(409, { code: "AlreadyReceived", message: "" }),
    );
    await expect(submitBallot("m", ballot, noSleep)).resolves.toBeUndefined();
  });

  it("retries network and server errors, then succeeds", async () => {
    let calls = 0;
    vi.stubGlobal("fetch", async () => {
      calls++;
      if (calls === 1) throw new TypeError("network down");
      if (calls === 2)
        return jsonResponse(502, { code: "Internal", message: "" });
      return jsonResponse(204);
    });
    await submitBallot("m", ballot, noSleep);
    expect(calls).toBe(3);
  });

  it("does not retry a rejected ballot", async () => {
    let calls = 0;
    vi.stubGlobal("fetch", async () => {
      calls++;
      return jsonResponse(400, { code: "InvalidBallot", message: "bad" });
    });
    await expect(submitBallot("m", ballot, noSleep)).rejects.toMatchObject({
      code: "InvalidBallot",
    });
    expect(calls).toBe(1);
  });
});
