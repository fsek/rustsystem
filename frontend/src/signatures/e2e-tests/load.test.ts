/**
 * @vitest-environment node
 *
 * Load / concurrency tests for the full voting system.
 *
 * Three scenarios:
 *
 *   1. Concurrent host invites (N_MEETINGS × VOTERS_PER_MEETING)
 *      Multiple meetings are created simultaneously, then every host
 *      concurrently adds all its voters in a single Promise.all wave.
 *      Verifies there are no lock-ordering bugs or races in the host
 *      invite path when meetings are written in parallel.
 *
 *   2. 50 simultaneous registrations
 *      One meeting, 50 logged-in voters, all fire POST /api/register at the
 *      same time. Checks that trustauth's duplicate-prevention guard and the
 *      shared RoundState lock hold up under concurrent load.
 *
 *   3. 200-voter end-to-end flow
 *      Full protocol at scale: create meeting → add 200 voters → log in all
 *      200 (batched) → start vote → all 200 register simultaneously → all
 *      200 get vote data and submit simultaneously → tally → verify counts.
 *
 * Why ConcurrentClient instead of TestClient?
 *   TestClient.withSession() patches globalThis.fetch to inject cookies and
 *   rewrite relative URLs. That design is safe when only one client is active
 *   at a time, but breaks under concurrency: two clients racing on withSession()
 *   will overwrite each other's global fetch. ConcurrentClient avoids the
 *   global patch entirely — it calls fetch directly with explicit absolute URLs
 *   and keeps its own per-instance cookie jar.
 *
 * Requires both services running:
 *   cargo run --bin rustsystem-server
 *   cargo run --bin rustsystem-trustauth
 *   pnpm test src/signatures/e2e-tests/load.test.ts
 */

import { generateToken, buildBallot, uuidToBytes } from "../signatures";
import type { BallotMetaData } from "../signatures";
import { BASE_URL, TRUSTAUTH_URL, DEFAULT_METADATA } from "./helpers";
import type { TallyResult, VoteData } from "../voteSession";

// ── Service reachability ───────────────────────────────────────────────────────

const servicesReachable: boolean = await Promise.all([
  fetch(`${BASE_URL}/`)
    .then(() => true)
    .catch(() => false),
  fetch(`${TRUSTAUTH_URL}/api/is-registered`)
    .then(() => true)
    .catch(() => false),
]).then(([s, t]) => s && t);

// ── ConcurrentClient ──────────────────────────────────────────────────────────

/**
 * A self-contained, concurrency-safe HTTP client with its own cookie jar.
 *
 * Unlike TestClient it does NOT patch globalThis.fetch. Every method calls
 * the native fetch directly using explicit absolute URLs, so many
 * ConcurrentClient instances can run in parallel without interfering.
 */
class ConcurrentClient {
  private cookieJar = "";

  private storeCookies(headers: Headers): void {
    // Node 20.10+ exposes getSetCookie(); fall back to splitting the combined
    // header string (older runtimes / undici behaviour).
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
      this.cookieJar = this.cookieJar
        ? re.test(this.cookieJar)
          ? this.cookieJar.replace(re, nameValue)
          : `${this.cookieJar}; ${nameValue}`
        : nameValue;
    }
  }

  /** Send a request, injecting stored cookies and persisting Set-Cookie headers. */
  private async req(url: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers as HeadersInit | undefined);
    if (!headers.has("Content-Type"))
      headers.set("Content-Type", "application/json");
    if (this.cookieJar) headers.set("Cookie", this.cookieJar);

    const res = await fetch(url, { ...init, headers });
    this.storeCookies(res.headers);
    return res;
  }

  // ── Meeting setup ────────────────────────────────────────────────────────────

  /**
   * POST /api/create-meeting (server) then POST /api/login (trustauth).
   * Returns the session IDs for subsequent calls.
   */
  async createMeeting(
    title = "Load Test",
    hostName = "Test Host",
  ): Promise<{ muuid: string; uuuid: string }> {
    const serverRes = await this.req(`${BASE_URL}/api/create-meeting`, {
      method: "POST",
      body: JSON.stringify({ title, host_name: hostName, pub_key: "test-key" }),
    });
    if (!serverRes.ok)
      throw new Error(`createMeeting HTTP ${serverRes.status}`);

    const data = await serverRes.json();
    const ids = { muuid: data.muuid as string, uuuid: data.uuuid as string };

    const trustRes = await this.req(`${TRUSTAUTH_URL}/api/login`, {
      method: "POST",
      body: JSON.stringify(ids),
    });
    if (!trustRes.ok)
      throw new Error(`trustauth createMeeting login HTTP ${trustRes.status}`);

    return ids;
  }

  /**
   * POST /api/host/new-voter — add a voter and return the invite link.
   * Requires a host session (cookie set by createMeeting).
   */
  async addVoter(name: string, isHost = false): Promise<string> {
    const res = await this.req(`${BASE_URL}/api/host/new-voter`, {
      method: "POST",
      body: JSON.stringify({ voterName: name, isHost }),
    });
    if (!res.ok) throw new Error(`addVoter HTTP ${res.status}`);
    const data = await res.json();
    return data.inviteLink as string;
  }

  /**
   * Log in by following an invite link (voter or host).
   *
   * Host invite links carry admin_msg (hex-encoded bytes) and admin_sig as
   * extra query parameters. When present they are forwarded as admin_cred in
   * the login body, causing the server to issue a host JWT (is_host=true).
   * Without them a regular voter JWT is issued. Mirrors the logic in login.tsx.
   */
  async loginFromInviteLink(
    inviteLink: string,
  ): Promise<{ muuid: string; uuuid: string }> {
    // The invite link may be a full URL or a relative path; URL() handles both.
    const url = new URL(inviteLink, BASE_URL);
    const muuid = url.searchParams.get("muuid")!;
    const uuuid = url.searchParams.get("uuuid")!;
    const adminMsg = url.searchParams.get("admin_msg");
    const adminSig = url.searchParams.get("admin_sig");
    const admin_cred =
      adminMsg && adminSig
        ? { msg: adminMsg.match(/.{2}/g)!.map((b) => parseInt(b, 16)), sig: adminSig }
        : undefined;

    const serverRes = await this.req(`${BASE_URL}/api/login`, {
      method: "POST",
      body: JSON.stringify({ uuuid, muuid, admin_cred }),
    });
    if (!serverRes.ok)
      throw new Error(`voter server login HTTP ${serverRes.status}`);

    const trustRes = await this.req(`${TRUSTAUTH_URL}/api/login`, {
      method: "POST",
      body: JSON.stringify({ uuuid, muuid }),
    });
    if (!trustRes.ok)
      throw new Error(`voter trustauth login HTTP ${trustRes.status}`);

    return { muuid, uuuid };
  }

  // ── Vote round management ────────────────────────────────────────────────────

  async startVoteRound(
    name = "Load Test Vote",
    metadata: BallotMetaData = DEFAULT_METADATA,
  ): Promise<void> {
    const res = await this.req(`${BASE_URL}/api/host/start-vote`, {
      method: "POST",
      body: JSON.stringify({ name, shuffle: false, metadata }),
    });
    if (!res.ok) throw new Error(`startVoteRound HTTP ${res.status}`);
  }

  // ── Voter workflow ────────────────────────────────────────────────────────────

  /**
   * POST /api/register on trustauth — generate a blind-signature commitment
   * and get back a blind signature. The token and blind_factor are derived
   * from the voter's session IDs and kept in memory for later ballot building.
   *
   * Stores the generated token/blind_factor in this client so getVoteData()
   * can serve them from the trustauth /api/vote-data endpoint.
   */
  async registerVoter(session: {
    uuuid: string;
    muuid: string;
  }): Promise<void> {
    const tokenData = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await this.req(`${TRUSTAUTH_URL}/api/register`, {
      method: "POST",
      body: JSON.stringify({
        context: tokenData.context,
        commitment: tokenData.commitmentJson,
        token: Array.from(tokenData.token),
        blind_factor: Array.from(tokenData.blindFactor),
      }),
    });
    if (!res.ok) throw new Error(`registerVoter HTTP ${res.status}`);
  }

  /**
   * GET /api/vote-data — retrieve the stored token, blind_factor, and blind
   * signature from trustauth. Must be called after registerVoter.
   */
  async getVoteData(): Promise<VoteData> {
    const res = await this.req(`${TRUSTAUTH_URL}/api/vote-data`);
    if (!res.ok) throw new Error(`getVoteData HTTP ${res.status}`);
    return res.json() as Promise<VoteData>;
  }

  /**
   * Build a ballot from the stored vote data and POST it to /api/voter/submit.
   * @param choice  Array of candidate indices, or null for a blank vote.
   */
  async submitVote(
    voteData: VoteData,
    choice: number[] | null,
    metadata: BallotMetaData = DEFAULT_METADATA,
  ): Promise<void> {
    const ballot = buildBallot(
      metadata,
      choice,
      new Uint8Array(voteData.token),
      new Uint8Array(voteData.blind_factor),
      voteData.signature,
    );
    const res = await this.req(`${BASE_URL}/api/voter/submit`, {
      method: "POST",
      body: JSON.stringify(ballot),
    });
    if (!res.ok) throw new Error(`submitVote HTTP ${res.status}`);
  }

  // ── Tally & cleanup ──────────────────────────────────────────────────────────

  async endVoteRound(): Promise<void> {
    const res = await this.req(`${BASE_URL}/api/host/end-vote-round`, {
      method: "DELETE",
    });
    if (!res.ok) throw new Error(`endVoteRound HTTP ${res.status}`);
  }

  async tally(): Promise<TallyResult> {
    const res = await this.req(`${BASE_URL}/api/host/tally`, {
      method: "POST",
    });
    if (!res.ok) throw new Error(`tally HTTP ${res.status}`);
    return res.json() as Promise<TallyResult>;
  }

  async closeMeeting(): Promise<void> {
    await this.req(`${BASE_URL}/api/host/close-meeting`, { method: "DELETE" });
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Execute `tasks` with at most `batchSize` tasks running concurrently.
 * Preserves insertion order in the returned results array.
 */
async function runInBatches<T>(
  tasks: Array<() => Promise<T>>,
  batchSize: number,
): Promise<T[]> {
  const results: T[] = [];
  for (let i = 0; i < tasks.length; i += batchSize) {
    const wave = tasks.slice(i, i + batchSize).map((fn) => fn());
    results.push(...(await Promise.all(wave)));
  }
  return results;
}

// ── Test suites ────────────────────────────────────────────────────────────────

// ─── 1. Concurrent host invites ────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("concurrent host invites", () => {
  /**
   * Five meetings are created simultaneously; then every host adds twenty
   * voters at the same time, giving 100 parallel new-voter requests spread
   * across five independent meetings. We verify:
   *   • all 100 calls succeed (no 5xx or deadlock)
   *   • every returned invite link contains the expected query parameters
   *
   * This exercises write-path concurrency: the outer AppState read-lock is
   * held briefly to clone each Arc<Meeting>, then per-meeting voter locks are
   * acquired independently. Parallel writes to different meetings must not
   * interfere.
   */
  it(
    "5 hosts each add 20 voters concurrently — all 100 invite links are valid",
    async () => {
      const N_MEETINGS = 5;
      const VOTERS_PER_MEETING = 20;

      // ── Create all meetings concurrently ────────────────────────────────────
      const hosts = await Promise.all(
        Array.from({ length: N_MEETINGS }, async (_, i) => {
          const host = new ConcurrentClient();
          await host.createMeeting(`Concurrent Meeting ${i + 1}`, `Host ${i + 1}`);
          return host;
        }),
      );

      // ── Every host adds all its voters simultaneously ────────────────────────
      // Flatten to a single Promise.all so all 100 requests fire at once.
      const allTasks = hosts.flatMap((host, hi) =>
        Array.from({ length: VOTERS_PER_MEETING }, (_, vi) => () =>
          host.addVoter(`M${hi + 1} Voter ${vi + 1}`),
        ),
      );
      const inviteLinks = await Promise.all(allTasks.map((fn) => fn()));

      // ── Assertions ──────────────────────────────────────────────────────────
      expect(inviteLinks).toHaveLength(N_MEETINGS * VOTERS_PER_MEETING);
      for (const link of inviteLinks) {
        expect(link).toMatch(/\/login\?/);
        expect(link).toMatch(/muuid=/);
        expect(link).toMatch(/uuuid=/);
      }

      // ── Cleanup ─────────────────────────────────────────────────────────────
      await Promise.all(hosts.map((host) => host.closeMeeting()));
    },
    { timeout: 60_000 },
  );
});

// ─── 2. 50 simultaneous registrations ─────────────────────────────────────────

describe.skipIf(!servicesReachable)("50 simultaneous registrations", () => {
  /**
   * Fifty voters all call POST /api/register on trustauth at the same time.
   *
   * The trustauth RoundState holds an async mutex that guards the registered-
   * voters set. This test checks that:
   *   • all 50 registrations complete successfully under contention
   *   • no voter is incorrectly rejected as a duplicate (each client has a
   *     distinct JWT, so trustauth must serialise the set writes correctly)
   *
   * After registration we start a tally to confirm the server session is still
   * consistent (tally count = 0 since no votes were submitted).
   */
  it(
    "all 50 registrations succeed with no duplicates or errors",
    async () => {
      const N = 50;

      // ── Setup: one meeting, N voters ────────────────────────────────────────
      const host = new ConcurrentClient();
      await host.createMeeting("50-Reg Load Test", "Host");

      // Add all voters via host API (parallel — exercises write path too)
      const inviteLinks = await Promise.all(
        Array.from({ length: N }, (_, i) => host.addVoter(`Voter ${i + 1}`)),
      );

      // Log in all voters before start-vote so they are not pruned.
      // Use a batch size of 25 to avoid saturating the login endpoint.
      const voterSessions = await runInBatches(
        inviteLinks.map((link) => async () => {
          const voter = new ConcurrentClient();
          const session = await voter.loginFromInviteLink(link);
          return { voter, session };
        }),
        25,
      );

      // ── Start vote round ────────────────────────────────────────────────────
      await host.startVoteRound("Concurrent Registration Test");

      // ── All 50 voters register simultaneously ────────────────────────────────
      // This is the core of the test: 50 concurrent writes to the trustauth
      // registered-voters set. Every one must succeed (no AlreadyRegistered 409).
      await Promise.all(
        voterSessions.map(({ voter, session }) => voter.registerVoter(session)),
      );

      // ── Tally: no votes submitted, all counts must be zero ──────────────────
      const result = await host.tally();
      expect(result.blank).toBe(0);
      for (const candidate of DEFAULT_METADATA.candidates) {
        expect(result.score[candidate]).toBe(0);
      }

      // ── Cleanup ─────────────────────────────────────────────────────────────
      await host.closeMeeting();
    },
    { timeout: 3 * 60_000 },
  );
});

// ─── 3. 500-voter multi-round stress test ─────────────────────────────────────

describe.skipIf(!servicesReachable)("500-voter full system flow", () => {
  /**
   * The entire voting protocol under maximum concurrency with 500 participants
   * across three consecutive vote rounds.
   *
   * Every step is a single Promise.all — no batching anywhere. The server and
   * trustauth face worst-case burst load at every phase, and the state machine
   * must transition cleanly through Idle → Voting → Tally → Idle three times.
   *
   * Multi-host invite phase (runs once, before all rounds):
   *   One master host creates the meeting and promotes 9 co-hosts. All 10 hosts
   *   then add their 50 voters simultaneously (500 concurrent new-voter writes
   *   across 10 host JWTs on the same meeting). All 500 voters log in at once
   *   before start-vote prunes unclaimed slots.
   *
   * Per-round phase (repeated for each round):
   *   start-vote → 500 register simultaneously → 500 get-vote-data simultaneously
   *   → 500 submit simultaneously → tally → assert exact counts → end-vote-round
   *
   * Round distributions (500 voters each):
   *   Round 1 — mixed:     125 blank / 150 A / 125 B / 100 C
   *   Round 2 — no blanks:   0 blank / 200 A / 150 B / 150 C
   *   Round 3 — all blank: 500 blank /   0 A /   0 B /   0 C
   *
   * State machine transitions under stress:
   *   Idle →(start-vote)→ Voting →(tally)→ Tally →(end-vote-round)→ Idle  ×3
   *   A second start-vote while Voting must 409; tallying while Idle must 410.
   *   Both are verified after every round transition.
   */
  it(
    "500 voters across 3 rounds — state machine stays consistent, tallies match",
    async () => {
      const N = 500;
      const N_COHOSTS = 9; // + 1 master = 10 total host clients
      const N_HOSTS = N_COHOSTS + 1;
      const VOTERS_PER_HOST = N / N_HOSTS; // 50 voters per host

      // Per-round vote distributions. choices[i] is indexed by voter position
      // in the flat invite-link array produced during setup.
      const ROUNDS = [
        {
          name: "Round 1 — mixed",
          choices: [
            ...Array<null>(125).fill(null),     // blank
            ...Array<number[]>(150).fill([0]),  // Option A
            ...Array<number[]>(125).fill([1]),  // Option B
            ...Array<number[]>(100).fill([2]),  // Option C
          ] as Array<number[] | null>,
          expected: { blank: 125, A: 150, B: 125, C: 100 },
        },
        {
          name: "Round 2 — no blanks",
          choices: [
            ...Array<number[]>(200).fill([0]),  // Option A
            ...Array<number[]>(150).fill([1]),  // Option B
            ...Array<number[]>(150).fill([2]),  // Option C
          ] as Array<number[] | null>,
          expected: { blank: 0, A: 200, B: 150, C: 150 },
        },
        {
          name: "Round 3 — all blank",
          choices: Array<null>(N).fill(null) as Array<number[] | null>,
          expected: { blank: 500, A: 0, B: 0, C: 0 },
        },
      ];

      // ── 1. Master creates the meeting ──────────────────────────────────────
      const master = new ConcurrentClient();
      await master.createMeeting("500-Voter Multi-Round Test", "Master Host");

      // ── 2. Master creates 9 co-host invite links concurrently ──────────────
      const coHostLinks = await Promise.all(
        Array.from({ length: N_COHOSTS }, (_, i) =>
          master.addVoter(`Co-Host ${i + 1}`, /* isHost */ true),
        ),
      );

      // ── 3. All co-hosts log in concurrently ────────────────────────────────
      const coHosts = await Promise.all(
        coHostLinks.map(async (link) => {
          const client = new ConcurrentClient();
          await client.loginFromInviteLink(link);
          return client;
        }),
      );

      const allHosts = [master, ...coHosts]; // 10 host clients

      // ── 4. All 10 hosts add their voters simultaneously ────────────────────
      // 500 concurrent new-voter writes across 10 host JWTs on the same meeting.
      const inviteLinks = (
        await Promise.all(
          allHosts.map((host, hi) =>
            Promise.all(
              Array.from({ length: VOTERS_PER_HOST }, (_, vi) =>
                host.addVoter(`H${hi + 1} Voter ${vi + 1}`),
              ),
            ),
          ),
        )
      ).flat();
      expect(inviteLinks).toHaveLength(N);

      // ── 5. All 500 voters log in simultaneously ────────────────────────────
      // 1 000 HTTP calls in-flight at once (server login + trustauth login per voter).
      // Must finish before start-vote, which prunes unclaimed voter slots.
      const voterSessions = await Promise.all(
        inviteLinks.map(async (link) => {
          const voter = new ConcurrentClient();
          const session = await voter.loginFromInviteLink(link);
          return { voter, session };
        }),
      );
      expect(voterSessions).toHaveLength(N);

      // ── 6. Three vote rounds ───────────────────────────────────────────────
      // ── State machine guard: tally while Idle must fail ───────────────────
      // Verified once before any round to keep the per-round hot path clean.
      // Starting a round is tested below; duplicate-start is checked BEFORE the
      // round goes live so it can never corrupt an active keypair.
      const idleTally = await master["req"](`${BASE_URL}/api/host/tally`, { method: "POST" });
      expect(idleTally.status).toBe(410); // VotingInactive — no active round yet

      for (const [ri, round] of ROUNDS.entries()) {
        // ── 6a. Start round ─────────────────────────────────────────────────
        await master.startVoteRound(round.name);

        // Duplicate start-vote while Voting must be rejected (409).
        // NOTE: this check is safe post-fix because the server now acquires the
        // vote_auth write-lock BEFORE calling trustauth, so a rejected duplicate
        // never reaches trustauth and cannot replace the active keypair.
        const dupStart = await master["req"](
          `${BASE_URL}/api/host/start-vote`,
          { method: "POST", body: JSON.stringify({ name: "dup", shuffle: false, metadata: DEFAULT_METADATA }) },
        );
        expect(dupStart.status).toBe(409);

        // ── 6b. All 500 register simultaneously ─────────────────────────────
        await Promise.all(
          voterSessions.map(({ voter, session }) =>
            voter.registerVoter(session),
          ),
        );

        // ── 6c. All 500 get vote data simultaneously ─────────────────────────
        const voteDataList = await Promise.all(
          voterSessions.map(({ voter }) => voter.getVoteData()),
        );

        // ── 6d. All 500 submit simultaneously ────────────────────────────────
        await Promise.all(
          voterSessions.map(({ voter }, i) =>
            voter.submitVote(voteDataList[i], round.choices[i]),
          ),
        );

        // ── 6e. Tally and verify ─────────────────────────────────────────────
        const result = await master.tally();

        expect(result.blank).toBe(round.expected.blank);
        expect(result.score[DEFAULT_METADATA.candidates[0]]).toBe(round.expected.A);
        expect(result.score[DEFAULT_METADATA.candidates[1]]).toBe(round.expected.B);
        expect(result.score[DEFAULT_METADATA.candidates[2]]).toBe(round.expected.C);

        const total =
          result.blank +
          Object.values(result.score).reduce((a, b) => a + b, 0);
        expect(total).toBe(N);

        // Tallying again from Tally state must be rejected (410).
        const dupTally = await master["req"](
          `${BASE_URL}/api/host/tally`,
          { method: "POST" },
        );
        expect(dupTally.status).toBe(410);

        // ── 6f. Reset state machine for the next round ───────────────────────
        await master.endVoteRound();

        // Tallying after reset (Idle state) must also be rejected (410).
        const staleTally = await master["req"](
          `${BASE_URL}/api/host/tally`,
          { method: "POST" },
        );
        expect(staleTally.status).toBe(410);
      }

      // ── 7. Cleanup ──────────────────────────────────────────────────────────
      await master.closeMeeting();
    },
    { timeout: 45 * 60_000 }, // 45 minutes — 3 rounds × 500 concurrent voters
  );
});
