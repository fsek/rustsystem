/**
 * Shared helpers for e2e tests.
 *
 * These tests require a live server. Set E2E_API_URL to override the default:
 *   E2E_API_URL=http://localhost:3000 pnpm test
 *
 * Why this file exists:
 *   voteSession.ts uses `apiFetch`, which calls the global `fetch` with relative
 *   URLs (e.g. "/api/create-meeting") and `credentials: "include"`. Both of those
 *   work correctly in a browser:
 *     • Relative URLs are resolved against the page origin.
 *     • `credentials: "include"` causes the browser to attach its cookie jar.
 *
 *   In Node.js neither behaviour is automatic. This file solves both problems by
 *   temporarily patching `globalThis.fetch` around each voteSession call so that:
 *     1. Relative URLs are made absolute using BASE_URL.
 *     2. The stored session cookie is injected into every outgoing request.
 *     3. Set-Cookie headers in every response are captured and stored.
 *
 *   All actual API logic (request bodies, error handling, crypto) lives in
 *   voteSession.ts and is reused here without modification.
 */

import {
  createMeeting as _createMeeting,
  startVoteRound as _startVoteRound,
  endVoteRound as _endVoteRound,
  tally as _tally,
  getTally as _getTally,
  registerVoter as _registerVoter,
  submitVote as _submitVote,
  apiFetch,
  type SessionIds,
  type RegisterResult,
  type TallyResult,
} from "../voteSession";
import type {
  GeneratedToken,
  RegistrationSuccessResponse,
  BallotMetaData,
} from "../signatures";

export { type TallyResult };

// biome-ignore lint/suspicious/noExplicitAny: process is only available in Node/vitest
export const BASE_URL: string =
  (globalThis as any).process?.env?.["E2E_API_URL"] ?? "http://localhost:3000";

export const DEFAULT_METADATA: BallotMetaData = {
  candidates: ["Option A", "Option B", "Option C"],
  max_choices: 1,
  protocol_version: 1,
};

// ─── TestClient ───────────────────────────────────────────────────────────────

export class TestClient {
  private cookieHeader = "";

  /**
   * Core cookie-aware request wrapper.
   *
   * Temporarily replaces `globalThis.fetch` with a version that:
   *   - Prepends BASE_URL to relative paths (e.g. "/api/…" → "http://localhost:3000/api/…")
   *   - Sends the stored Cookie header on every request
   *   - Stores Set-Cookie headers from every response
   *
   * The original `fetch` is always restored in `finally`, so a thrown error
   * inside `fn` never leaves the global fetch permanently patched.
   *
   * Tests within a file run sequentially, so patching the global is safe.
   */
  private async withSession<T>(fn: () => Promise<T>): Promise<T> {
    const original = globalThis.fetch;
    const client = this;

    globalThis.fetch = async (
      input: RequestInfo | URL,
      init?: RequestInit,
    ): Promise<Response> => {
      // Resolve relative URLs against the test server base
      const raw =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.href
            : (input as Request).url;
      const url = raw.startsWith("/") ? `${BASE_URL}${raw}` : raw;

      const headers = new Headers(init?.headers as HeadersInit | undefined);
      if (client.cookieHeader) headers.set("Cookie", client.cookieHeader);

      const res = await original(url, { ...init, headers });
      client.storeCookies(res.headers);
      return res;
    };

    try {
      return await fn();
    } finally {
      globalThis.fetch = original;
    }
  }

  private storeCookies(headers: Headers): void {
    // Use getSetCookie() (Node 20.10+ / undici) when available;
    // fall back to splitting the combined header string on newlines.
    type H = { getSetCookie?(): string[] };
    const rawList: string[] =
      typeof (headers as unknown as H).getSetCookie === "function"
        ? (headers as unknown as H).getSetCookie!()
        : (headers.get("set-cookie") ?? "").split(/\n/).filter(Boolean);

    for (const raw of rawList) {
      const nameValue = raw.split(";")[0].trim();
      if (!nameValue.includes("=")) continue;
      const name = nameValue.split("=")[0];
      const re = new RegExp(`(?:^|; *)${name}=[^;]*`);
      this.cookieHeader = this.cookieHeader
        ? re.test(this.cookieHeader)
          ? this.cookieHeader.replace(re, nameValue)
          : `${this.cookieHeader}; ${nameValue}`
        : nameValue;
    }
  }

  // ── Wrappers around voteSession functions ─────────────────────────────────

  createMeeting(title = "Test Meeting", hostName = "Test Host", pubKey = "test-key") {
    return this.withSession(() => _createMeeting(title, hostName, pubKey));
  }

  startVoteRound(
    name = "Test Vote",
    shuffle = false,
    metadata: BallotMetaData = DEFAULT_METADATA,
  ) {
    return this.withSession(() => _startVoteRound(name, shuffle, metadata));
  }

  endVoteRound() {
    return this.withSession(() => _endVoteRound());
  }

  tally() {
    return this.withSession(() => _tally());
  }

  getTally() {
    return this.withSession(() => _getTally());
  }

  registerVoter(session: SessionIds): Promise<RegisterResult> {
    return this.withSession(() => _registerVoter(session));
  }

  submitVote(
    token: GeneratedToken,
    regResponse: RegistrationSuccessResponse,
    choice: number[] | null,
    metadata: BallotMetaData = DEFAULT_METADATA,
  ) {
    return this.withSession(() => _submitVote(token, regResponse, metadata, choice));
  }

  /**
   * Low-level method for negative tests that need to inspect the raw Response
   * (status code, error body) rather than having the helper throw on failure.
   * Uses apiFetch directly so the cookie and URL logic is still applied.
   */
  rawRequest(method: string, path: string, body?: unknown): Promise<Response> {
    return this.withSession(() =>
      apiFetch(path, {
        method,
        ...(body !== undefined && { body: JSON.stringify(body) }),
      }),
    );
  }
}

// ─── Signature corruption ─────────────────────────────────────────────────────

/**
 * Return a copy of `signature` with one scalar byte changed.
 *
 * A BBS+ blind signature (zkryptium / BbsBls12381Sha256) serialises to JSON as:
 *   { "BBSplus": { "A": "<96 hex chars>", "e": "<64 hex chars>" } }
 *
 * We must corrupt the scalar `e` (64 chars), NOT the G1 point `A` (96 chars).
 * Flipping a character in `A` usually produces bytes that cannot decode to a
 * valid curve point, so the server rejects the request at deserialisation time
 * with 422 Unprocessable Entity — before any cryptographic check occurs.
 * Flipping a character in `e` leaves a structurally valid scalar, but one that
 * no longer satisfies the BBS+ equation, so the server returns 401.
 *
 * The regex /"([0-9a-f]{64})"/ matches EXACTLY 64-char hex strings.
 * `A` is 96 chars and therefore will not match, so we always land on `e`.
 */
export function corruptSignature(signature: unknown): unknown {
  const json = JSON.stringify(signature);

  // Find the first exactly-64-char hex string (the scalar field `e`).
  const match = /"([0-9a-f]{64})"/.exec(json);
  if (match?.index !== undefined) {
    const hexStart = match.index + 1; // position of first hex char (skip the opening quote)
    const mid = hexStart + 32; // flip the 17th byte (middle of the 32-byte scalar)
    const flipped = (((parseInt(json[mid], 16) + 1) & 0xf) >>> 0).toString(16);
    return JSON.parse(json.slice(0, mid) + flipped + json.slice(mid + 1));
  }

  throw new Error(
    "corruptSignature: could not find a 64-char hex scalar — " +
    "check the serialisation format from zkryptium",
  );
}
