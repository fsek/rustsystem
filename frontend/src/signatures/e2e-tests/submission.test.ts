/**
 * @vitest-environment node
 *
 * E2E tests for POST /api/voter/submit.
 *
 * Submission is the final step of the anonymous voting protocol. The server must
 * verify the blind signature without learning which voter cast which ballot, and
 * it must prevent the same signature from being reused (replay attack).
 *
 * Tests cover:
 *  - Happy path (candidate vote, blank vote, multi-choice)
 *  - State constraints (vote must be active)
 *  - Signature attacks (fresh token + old sig, bit-flipped sig, replay)
 *  - Ballot metadata validation (wrong metadata, too many choices)
 *  - Requests without a valid session cookie
 *
 * Requires a live server. Run with:
 *   API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
 *   pnpm test src/signatures/e2e/submission.test.ts
 */

import { generateToken, buildBallot, uuidToBytes } from "../signatures";
import {
  TestClient,
  BASE_URL,
  DEFAULT_METADATA,
  corruptSignature,
} from "./helpers";

const serverReachable: boolean = await fetch(`${BASE_URL}/`)
  .then(() => true)
  .catch(() => false);

// ─── Happy path ───────────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("submission — happy path", () => {
  it("accepts a vote for a single candidate", async () => {
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    await expect(
      client.submitVote(token, regResponse, [0]),
    ).resolves.toBeUndefined();

    // Tally to confirm the vote was counted
    const result = await client.tally();
    expect(result.score[DEFAULT_METADATA.candidates[0]]).toBe(1);
    expect(result.blank).toBe(0);
  });

  it("accepts a blank vote (null choice)", async () => {
    // A null choice is a legitimate "abstain" vote that the server records
    // separately in the blank counter. Voters must be able to submit without
    // choosing a candidate.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    await expect(
      client.submitVote(token, regResponse, null),
    ).resolves.toBeUndefined();

    const result = await client.tally();
    expect(result.blank).toBe(1);
  });

  it("accepts multiple choices when max_choices > 1", async () => {
    // When the round allows more than one selection the voter may include
    // several candidate indices in a single ballot.
    const multiMeta = {
      candidates: ["A", "B", "C"],
      max_choices: 2,
      protocol_version: 1,
    };
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound("multi-choice round", false, multiMeta);
    const { token, regResponse } = await client.registerVoter(session);

    await expect(
      client.submitVote(token, regResponse, [0, 2]),
    ).resolves.toBeUndefined();

    const result = await client.tally();
    expect(result.score["A"]).toBe(1);
    expect(result.score["C"]).toBe(1);
  });
});

// ─── State constraints ────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("submission — state constraints", () => {
  it("fails with 410 when no vote round is active", async () => {
    // Even a valid blind signature must be rejected if the round is no longer
    // open — submitting after the deadline would corrupt the tally.
    //
    // We register during an active round, end it, then attempt to submit.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);
    await client.endVoteRound(); // round is now closed

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      regResponse.signature,
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(410); // VotingInactive
  });

  it("fails with 410 after the round has been tallied", async () => {
    // Once the host calls tally, voting is closed. Any subsequent submission
    // must be rejected regardless of signature validity.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);
    await client.tally();

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      regResponse.signature,
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(410); // VotingInactive
  });
});

// ─── Signature attacks ────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("submission — signature attacks", () => {
  it("fails with 401 when a fresh token is paired with an old signature", async () => {
    // The signature was blind-signed for the commitment of `token`. Submitting
    // `token2` (a completely different random value) with that signature must fail
    // because the BBS+ verification binds the signature to the original committed
    // token. This validates the unlinkability guarantee: you cannot forge a valid
    // ballot by reusing a legitimate signature with a different token.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { regResponse } = await client.registerVoter(session);

    // Generate a brand-new token — this was NOT committed to during registration
    const freshToken = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      freshToken.token, // ← wrong token (not the one that was signed)
      freshToken.blindFactor, // ← blind factor for the wrong token
      regResponse.signature, // ← signature from the original registration
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(401); // SignatureInvalid
  });

  it("fails with 401 when a single byte of the scalar field is flipped", async () => {
    // BbsBls12381Sha256 blind signatures serialise as { A: <G1, 96 hex>, e: <scalar, 64 hex> }.
    // corruptSignature targets the scalar `e` (exactly 64 hex chars), leaving the G1
    // point `A` intact so the structure still deserialises. The altered scalar
    // makes the signature fail BBS+ verification → 401 SignatureInvalid.
    //
    // (Corrupting `A` instead would produce an invalid curve point that Axum
    // cannot deserialise at all → 422 Unprocessable Entity, tested separately.)
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    const tamperedSignature = corruptSignature(regResponse.signature);
    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      tamperedSignature, // ← scalar `e` has one hex digit changed
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(401); // SignatureInvalid
  });

  it("fails with 422 when the signature is structurally malformed", async () => {
    // The submit endpoint deserialises the ballot body via Axum's JSON extractor
    // before any application logic runs. Sending a value that cannot be decoded
    // as a BlindSignature (wrong type, missing fields, invalid curve point) causes
    // Axum to return 422 Unprocessable Entity.
    //
    // Concretely, we replace the signature with a plain string. The server cannot
    // construct a BBSplusSignature { A: G1Projective, e: Scalar } from a string,
    // so the request is rejected at the deserialisation layer, not the crypto layer.
    //
    // This distinguishes two failure modes that produce similar-looking errors:
    //   422 — the signature cannot even be parsed (malformed)
    //   401 — the signature parses fine but fails BBS+ verification (tampered)
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      "not-a-valid-signature", // ← completely wrong type for BlindSignature
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(422); // deserialization failure
  });

  it("fails with 409 when the same signature is submitted a second time (replay)", async () => {
    // The server marks a signature as "expired" immediately after the first
    // successful submission. Re-sending the identical ballot (same token, same
    // signature) must be rejected so a voter cannot count their vote twice.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    await client.submitVote(token, regResponse, [0]); // first submission: OK

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      regResponse.signature,
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(409); // SignatureExpired
  });
});

// ─── Ballot validation ────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("submission — ballot validation", () => {
  it("fails with 409 when the ballot metadata does not match the current round", async () => {
    // The server compares the metadata embedded in the ballot against the
    // metadata of the currently active round. A mismatch indicates either a
    // stale ballot from a previous round or a tampered submission.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    const { token, regResponse } = await client.registerVoter(session);

    const wrongMeta = {
      candidates: ["X", "Y"], // different candidates from the active round
      max_choices: 1,
      protocol_version: 1,
    };
    const ballot = buildBallot(
      wrongMeta, // ← metadata that doesn't match the server's round
      [0],
      token.token,
      token.blindFactor,
      regResponse.signature,
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(409); // InvalidMetaData
  });

  it("fails with 409 when the number of choices exceeds max_choices", async () => {
    // The round was started with max_choices=1. Sending two candidate indices
    // must be rejected so voters cannot gain an outsized influence on the result.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound(); // max_choices = 1
    const { token, regResponse } = await client.registerVoter(session);

    const ballot = buildBallot(
      regResponse.metadata,
      [0, 1], // ← 2 choices for a max_choices=1 round
      token.token,
      token.blindFactor,
      regResponse.signature,
    );
    const res = await client.rawRequest("POST", "/api/voter/submit", ballot);
    expect(res.status).toBe(409); // InvalidVoteLength
  });
});

// ─── Authentication ───────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("submission — authentication", () => {
  it("fails with 401 when no session cookie is present", async () => {
    // The submit endpoint requires a valid JWT cookie to identify the meeting.
    // Without it the AuthUser extractor fails before any ballot parsing occurs.
    //
    // We use a separate authenticated client to prepare a valid ballot so that
    // the test isolates the authentication check rather than body validation.
    const authClient = new TestClient();
    const session = await authClient.createMeeting();
    await authClient.startVoteRound();
    const { token, regResponse } = await authClient.registerVoter(session);

    const ballot = buildBallot(
      regResponse.metadata,
      [0],
      token.token,
      token.blindFactor,
      regResponse.signature,
    );

    // Submit with a fresh client that has no session cookie
    const unauthClient = new TestClient();
    const res = await unauthClient.rawRequest(
      "POST",
      "/api/voter/submit",
      ballot,
    );
    expect(res.status).toBe(401);
  });
});
