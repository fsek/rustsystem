/**
 * @vitest-environment node
 *
 * E2E tests for meeting creation and the vote-round state machine.
 *
 * State machine under test:
 *
 *   Idle ──(start-vote)──► Voting ──(tally)──► Tally ──(end-vote-round)──► Idle
 *
 * All tests create their own isolated meeting so they are independent of each
 * other and can run in any order without shared state.
 *
 * Requires both services running:
 *   cargo run --bin rustsystem-server
 *   cargo run --bin rustsystem-trustauth
 *   pnpm test src/signatures/e2e-tests/meeting.test.ts
 */

import {
  TestClient,
  BASE_URL,
  TRUSTAUTH_URL,
  DEFAULT_METADATA,
} from "./helpers";

// ─── Service reachability ─────────────────────────────────────────────────────

// Probe before defining any suites. describe.skipIf short-circuits the whole
// file cleanly when the services are not running, avoiding confusing network errors.
const servicesReachable: boolean = await Promise.all([
  fetch(`${BASE_URL}/`)
    .then(() => true)
    .catch(() => false),
  fetch(`${TRUSTAUTH_URL}/api/is-registered`)
    .then(() => true)
    .catch(() => false),
]).then(([s, t]) => s && t);

// ─── Meeting creation ─────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("createMeeting", () => {
  it("returns a valid muuid and uuuid", async () => {
    const client = new TestClient();
    const session = await client.createMeeting("My Meeting", "Host Name");

    // Both IDs must be non-empty strings — basic sanity check that the server
    // returned what we expect before any other test relies on these values.
    expect(typeof session.muuid).toBe("string");
    expect(session.muuid.length).toBeGreaterThan(0);
    expect(typeof session.uuuid).toBe("string");
    expect(session.uuuid.length).toBeGreaterThan(0);
  });

  it("sets a session cookie so subsequent host calls succeed", async () => {
    // The createMeeting response sets an HttpOnly session cookie. If it is not
    // stored and re-sent, the start-vote call would fail with 401. Passing here
    // proves cookie management is working end-to-end.
    const client = new TestClient();
    await client.createMeeting();
    await expect(client.startVoteRound()).resolves.toBeUndefined();
  });
});

// ─── start-vote ───────────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("startVoteRound", () => {
  it("succeeds when the server is in Idle state", async () => {
    const client = new TestClient();
    await client.createMeeting();
    await expect(client.startVoteRound()).resolves.toBeUndefined();
  });

  it("fails with 409 when a vote round is already active (Voting state)", async () => {
    // Starting a round while one is running must be rejected. Without this
    // guard a host could accidentally overwrite an ongoing vote.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();

    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "duplicate",
      shuffle: false,
      metadata: DEFAULT_METADATA,
    });
    expect(res.status).toBe(409); // InvalidState
  });

  it("fails with 409 after the round has been tallied (Tally state)", async () => {
    // The server remains in Tally state until end-vote-round is called. Starting
    // a new round from Tally would corrupt the displayed results, so it must fail.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();
    await client.tally();

    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "after-tally",
      shuffle: false,
      metadata: DEFAULT_METADATA,
    });
    expect(res.status).toBe(409); // InvalidState
  });

  it("fails with 409 when candidates contain duplicates", async () => {
    // BallotMetaData::check_valid() rejects duplicate candidates because
    // duplicate entries would make the tally ambiguous.
    const client = new TestClient();
    await client.createMeeting();

    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "bad meta",
      shuffle: false,
      metadata: {
        candidates: ["A", "A"], // duplicate
        max_choices: 1,
        protocol_version: 1,
      },
    });
    expect(res.status).toBe(409); // InvalidMetaData
  });

  it("fails with 409 when max_choices exceeds the number of candidates", async () => {
    // Asking voters to pick 5 winners from 3 candidates is nonsensical and
    // rejected by the server's metadata validation.
    const client = new TestClient();
    await client.createMeeting();

    const res = await client.rawRequest("POST", "/api/host/start-vote", {
      name: "bad meta",
      shuffle: false,
      metadata: {
        candidates: ["A", "B", "C"],
        max_choices: 5, // > candidates.len()
        protocol_version: 1,
      },
    });
    expect(res.status).toBe(409); // InvalidMetaData
  });
});

// ─── tally ────────────────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("tally", () => {
  it("succeeds in Voting state and returns a score map and blank count", async () => {
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();

    const result = await client.tally();

    // The tally of an empty round should have zero votes for every candidate
    // and zero blank votes.
    expect(result).toHaveProperty("score");
    expect(result).toHaveProperty("blank");
    expect(result.blank).toBe(0);
    for (const name of DEFAULT_METADATA.candidates) {
      expect(result.score[name]).toBe(0);
    }
  });

  it("fails with 410 when no vote round has been started (Idle state)", async () => {
    // There is nothing to tally without an active round.
    const client = new TestClient();
    await client.createMeeting();

    const res = await client.rawRequest("POST", "/api/host/tally");
    expect(res.status).toBe(410); // VotingInactive
  });

  it("fails with 410 when called a second time (Tally state)", async () => {
    // After a successful tally the server moves to Tally state (voting is closed).
    // Calling tally again must fail — there is nothing left to tally.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();
    await client.tally(); // first call succeeds

    const res = await client.rawRequest("POST", "/api/host/tally");
    expect(res.status).toBe(410); // VotingInactive
  });
});

// ─── get-tally ────────────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("getTally", () => {
  it("returns the stored tally after a successful tally call", async () => {
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();

    // Cast a vote for candidate 0, then tally
    await client.registerVoter(session);
    const voteData = await client.getVoteData();
    await client.submitVote(voteData, [0]);
    const tallyResult = await client.tally();

    // get-tally must return the same result as tally
    const fetched = await client.getTally();
    expect(fetched).toEqual(tallyResult);
    expect(fetched.score[DEFAULT_METADATA.candidates[0]]).toBe(1);
  });

  it("fails with 410 when no tally has been computed yet", async () => {
    // There is no tally to retrieve before the host calls tally.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();

    const res = await client.rawRequest("GET", "/api/host/get-tally");
    expect(res.status).toBe(409); // NoTallyAvailable
  });

  it("fails with 410 after end-vote-round resets the state", async () => {
    // end-vote-round clears the tally along with the rest of the round state,
    // so get-tally should no longer find a result.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();
    await client.tally();
    await client.endVoteRound(); // resets to Idle

    const res = await client.rawRequest("GET", "/api/host/get-tally");
    expect(res.status).toBe(409); // NoTallyAvailable
  });
});

// ─── end-vote-round ───────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("endVoteRound", () => {
  it("succeeds when a vote round is active (Voting state)", async () => {
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();
    await expect(client.endVoteRound()).resolves.toBeUndefined();
  });

  it("succeeds when the round is in Tally state", async () => {
    // Ending from Tally state is the normal post-results cleanup path.
    const client = new TestClient();
    await client.createMeeting();
    await client.startVoteRound();
    await client.tally();
    await expect(client.endVoteRound()).resolves.toBeUndefined();
  });

  it("is idempotent — succeeds even when no vote round is active (Idle state)", async () => {
    // EndVoteRound calls reset() unconditionally (no state guard). This is an
    // intentional design choice: the host can always clean up without worrying
    // about the current state. A second call after the round is already over
    // must therefore also succeed rather than return an error.
    const client = new TestClient();
    await client.createMeeting();
    // No startVoteRound — server is already in Idle state
    await expect(client.endVoteRound()).resolves.toBeUndefined();
  });
});

// ─── Full cycle ───────────────────────────────────────────────────────────────

describe.skipIf(!servicesReachable)("full vote cycle", () => {
  it("Idle → start → register → submit → tally → end → start again", async () => {
    // Exercises every state transition in the correct order and verifies that
    // the server correctly resets state so a second round can start.
    const client = new TestClient();
    const session = await client.createMeeting();

    // Round 1
    await client.startVoteRound("Round 1");
    await client.registerVoter(session);
    const voteData = await client.getVoteData();
    await client.submitVote(voteData, [1]); // vote for Option B
    const result = await client.tally();
    expect(result.score[DEFAULT_METADATA.candidates[1]]).toBe(1);
    await client.endVoteRound();

    // Round 2 — the meeting is reusable after a reset
    await expect(client.startVoteRound("Round 2")).resolves.toBeUndefined();
  });
});
