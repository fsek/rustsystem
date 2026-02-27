/**
 * Shared helpers for e2e tests.
 *
 * Two live services are required before these tests run:
 *   Server:    http://localhost:1443  (or override with E2E_API_URL)
 *   Trustauth: http://localhost:2443  (or override with E2E_TRUSTAUTH_URL)
 *
 * Why the TestClient reimplements trustauth calls:
 *   voteSession.ts derives trustauth URLs from `import.meta.env.API_ENDPOINT_TRUSTAUTH`,
 *   which is compiled in by Vite at startup time. In the Node.js/Vitest environment
 *   that value resolves to the Vite-dev-proxy path "/api/trustauth" (the new default),
 *   which is useless for direct Node.js HTTP calls. To avoid this, the TestClient
 *   implements the trustauth-bound methods (createMeeting, registerVoter, getVoteData)
 *   with explicit absolute URLs derived from TRUSTAUTH_URL, rather than calling
 *   through voteSession.ts's trustAuthFetch.
 *
 *   Server-only methods (startVoteRound, tally, etc.) still delegate to voteSession.ts
 *   because those functions always use relative paths that the TestClient routes
 *   to BASE_URL correctly.
 *
 * Cookie management:
 *   The server issues an `access_token` cookie and trustauth issues a `trustauth_token`
 *   cookie. Because they use different cookie names they coexist in the single shared
 *   cookie jar without conflict. The TestClient's `withSession` wrapper injects the
 *   combined cookie header into every outgoing request and stores Set-Cookie headers
 *   from every response, so both cookies are kept in sync automatically.
 */

import {
  startVoteRound as _startVoteRound,
  endVoteRound as _endVoteRound,
  tally as _tally,
  getTally as _getTally,
  submitVote as _submitVote,
  apiFetch,
  type SessionIds,
  type VoteData,
  type TallyResult,
} from "../voteSession";
import { generateToken, uuidToBytes, type BallotMetaData } from "../signatures";

export { type TallyResult };

// biome-ignore lint/suspicious/noExplicitAny: process is only available in Node/vitest
export const BASE_URL: string =
  (globalThis as any).process?.env?.["E2E_API_URL"] ?? "http://localhost:1443";

// biome-ignore lint/suspicious/noExplicitAny: process is only available in Node/vitest
export const TRUSTAUTH_URL: string =
  (globalThis as any).process?.env?.["E2E_TRUSTAUTH_URL"] ?? "http://localhost:2443";

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
   *   - Prepends BASE_URL to relative paths (e.g. "/api/…" → "http://localhost:1443/api/…")
   *   - Sends the stored Cookie header on every request (server + trustauth cookies)
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
      // Resolve relative URLs against the test server base.
      // Absolute URLs (e.g. explicit TRUSTAUTH_URL calls) are passed through as-is.
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

  // ── Meeting creation ────────────────────────────────────────────────────────

  /**
   * Create a meeting on the server and log in to trustauth.
   *
   * Implemented with explicit absolute URLs so the test environment does not
   * depend on import.meta.env.API_ENDPOINT_TRUSTAUTH being set at Vitest startup.
   *
   * The server issues an `access_token` cookie; trustauth issues a
   * `trustauth_token` cookie. Both are stored in the shared cookie jar.
   */
  async createMeeting(
    title = "Test Meeting",
    hostName = "Test Host",
    pubKey = "test-key",
  ): Promise<SessionIds> {
    // Step 1 — create meeting on server (receives access_token cookie)
    const serverRes = await this.withSession(() =>
      fetch(`${BASE_URL}/api/create-meeting`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title, host_name: hostName, pub_key: pubKey }),
      }),
    );
    const data = await serverRes.json();
    if (!serverRes.ok) throw new Error(`createMeeting HTTP ${serverRes.status}`);
    const ids: SessionIds = { uuuid: data.uuuid, muuid: data.muuid };

    // Step 2 — log in to trustauth (receives trustauth_token cookie)
    const trustRes = await this.withSession(() =>
      fetch(`${TRUSTAUTH_URL}/api/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ uuuid: ids.uuuid, muuid: ids.muuid }),
      }),
    );
    if (!trustRes.ok)
      throw new Error(`trustauth login HTTP ${trustRes.status}`);

    return ids;
  }

  // ── Vote-round management (delegated to voteSession.ts, server-only) ────────

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

  // ── Voter registration ──────────────────────────────────────────────────────

  /**
   * Register for the current vote round via trustauth.
   *
   * Uses an explicit absolute TRUSTAUTH_URL to avoid import.meta.env routing issues.
   * Requires the trustauth_token cookie (set by createMeeting).
   */
  async registerVoter(session: SessionIds): Promise<void> {
    const voterBytes = uuidToBytes(session.uuuid);
    const meetingBytes = uuidToBytes(session.muuid);
    const token = generateToken(voterBytes, meetingBytes);

    const body = {
      context: token.context,
      commitment: token.commitmentJson,
      token: Array.from(token.token),
      blind_factor: Array.from(token.blindFactor),
    };

    const res = await this.withSession(() =>
      fetch(`${TRUSTAUTH_URL}/api/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    );
    if (!res.ok) throw new Error(`registerVoter HTTP ${res.status}`);
  }

  // ── Vote data ───────────────────────────────────────────────────────────────

  /**
   * Retrieve the stored token, blind factor, and signature from trustauth.
   *
   * Uses an explicit absolute TRUSTAUTH_URL.
   * Requires the trustauth_token cookie.
   */
  async getVoteData(): Promise<VoteData> {
    const res = await this.withSession(() =>
      fetch(`${TRUSTAUTH_URL}/api/vote-data`, {
        headers: { "Content-Type": "application/json" },
      }),
    );
    const data = await res.json();
    if (!res.ok) throw new Error(`getVoteData HTTP ${res.status}`);
    return data as VoteData;
  }

  // ── Vote submission ─────────────────────────────────────────────────────────

  submitVote(
    voteData: VoteData,
    choice: number[] | null,
    metadata: BallotMetaData = DEFAULT_METADATA,
  ) {
    return this.withSession(() => _submitVote(voteData, metadata, choice));
  }

  // ── Raw requests ────────────────────────────────────────────────────────────

  /**
   * Low-level server request for negative tests.
   * Relative paths are resolved against BASE_URL (the server).
   */
  rawRequest(method: string, path: string, body?: unknown): Promise<Response> {
    return this.withSession(() =>
      apiFetch(path, {
        method,
        ...(body !== undefined && { body: JSON.stringify(body) }),
      }),
    );
  }

  /**
   * Low-level trustauth request for negative tests.
   * The URL is constructed as TRUSTAUTH_URL + path.
   */
  rawTrustAuthRequest(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<Response> {
    return this.withSession(() =>
      fetch(`${TRUSTAUTH_URL}${path}`, {
        method,
        headers: { "Content-Type": "application/json" },
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
