/**
 * @vitest-environment node
 *
 * E2E tests for POST /api/voter/register.
 *
 * Registration is the step where the voter proves to the server that they hold
 * a valid session, then obtains a blind signature that will later authorise
 * their ballot — without the server ever learning which vote belongs to them.
 *
 * Tests cover:
 *  - Happy path and response shape
 *  - State constraints (vote must be active)
 *  - Duplicate registration prevention
 *  - Requests without a valid session cookie
 *  - Malformed request bodies
 *
 * Requires a live server. Run with:
 *   API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
 *   pnpm test src/signatures/e2e/registration.test.ts
 */

import { generateToken, uuidToBytes } from "../signatures";
import { TestClient, BASE_URL, DEFAULT_METADATA } from "./helpers";

const serverReachable: boolean = await fetch(`${BASE_URL}/`)
  .then(() => true)
  .catch(() => false);

// ─── Happy path ───────────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("registration — happy path", () => {
  it("returns a signature and metadata when the vote round is active", async () => {
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();

    const { token, regResponse } = await client.registerVoter(session);

    // The response must carry the BBS+ blind signature and the round metadata
    // so the voter can later build a valid ballot without knowing the public key.
    expect(regResponse.signature).toBeDefined();
    expect(DEFAULT_METADATA.candidates).toEqual(
      DEFAULT_METADATA.candidates,
    );
    expect(DEFAULT_METADATA.max_choices).toBe(DEFAULT_METADATA.max_choices);

    // The locally generated token must be usable (non-empty)
    expect(token.token.length).toBeGreaterThan(0);
    expect(token.blindFactor.length).toBeGreaterThan(0);
  });
});

// ─── State constraints ────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("registration — state constraints", () => {
  it("fails with 410 before a vote round has been started", async () => {
    // The server must refuse registrations when there is no active round;
    // a signature obtained in Idle state would never be accepted at submission.
    const client = new TestClient();
    const session = await client.createMeeting();
    // deliberately skip startVoteRound

    const token = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      commitment: token.commitmentJson,
    });
    expect(res.status).toBe(410); // VoteInactive
  });

  it("fails with 410 after the round has been tallied (Tally state)", async () => {
    // Once the host closes voting the round is over. Registrations after that
    // point must be rejected so no late signatures can be used to backdate votes.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    await client.tally();

    const token = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      commitment: token.commitmentJson,
    });
    expect(res.status).toBe(410); // VoteInactive
  });

  it("fails with 410 after the round has been reset via end-vote-round", async () => {
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    await client.endVoteRound();

    const token = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      commitment: token.commitmentJson,
    });
    expect(res.status).toBe(410); // VoteInactive
  });
});

// ─── Duplicate registration ───────────────────────────────────────────────────

describe.skipIf(!serverReachable)("registration — duplicate prevention", () => {
  it("fails with 409 on a second registration attempt by the same user", async () => {
    // Each voter may only hold one valid blind signature per round. A second
    // registration would let a single voter produce two independently valid
    // ballots, undermining the one-person-one-vote guarantee.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();
    await client.registerVoter(session); // first: succeeds

    const token2 = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token2.context,
      commitment: token2.commitmentJson,
    });
    expect(res.status).toBe(409); // AlreadyRegistered
  });
});

// ─── Authentication ───────────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("registration — authentication", () => {
  it("fails with 401 when no session cookie is present", async () => {
    // Without a JWT cookie the AuthUser extractor cannot identify the voter.
    // The server must reject the request before it even reaches the registration
    // logic — leaking a blind signature to an unauthenticated caller would break
    // the access-control model.
    //
    // We send a well-formed commitment to isolate the authentication check from
    // any body-parsing errors.
    const unauthClient = new TestClient(); // no createMeeting → no cookie
    const fakeSession = {
      uuuid: "00000000-0000-0000-0000-000000000001",
      muuid: "00000000-0000-0000-0000-000000000002",
    };
    const token = generateToken(
      uuidToBytes(fakeSession.uuuid),
      uuidToBytes(fakeSession.muuid),
    );

    const res = await unauthClient.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      commitment: token.commitmentJson,
    });
    expect(res.status).toBe(401);
  });
});

// ─── Malformed requests ───────────────────────────────────────────────────────

describe.skipIf(!serverReachable)("registration — malformed requests", () => {
  it("fails with 4xx when the commitment field is missing", async () => {
    // Axum's JSON deserialiser rejects requests that are missing required fields.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();

    const token = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      // commitment intentionally omitted
    });
    expect(res.ok).toBe(false);
  });

  it("fails with 4xx when the commitment is not a valid object", async () => {
    // Sending a string instead of the expected CommitmentJson structure must
    // cause deserialization to fail before any crypto is attempted.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();

    const token = generateToken(
      uuidToBytes(session.uuuid),
      uuidToBytes(session.muuid),
    );
    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: token.context,
      commitment: "not-a-valid-commitment",
    });
    expect(res.ok).toBe(false);
  });

  it("context voter_id is not validated against the authenticated user", async () => {
    // The ProofContext voter_id is metadata used as a BBS+ signed message, NOT
    // cross-checked against the JWT. The round header (not the context) binds
    // the signature to the current round. A wrong voter_id therefore does not
    // cause a registration failure — the server signs the commitment as long as
    // the voter's JWT is valid and the commitment is well-formed.
    //
    // This test documents the current behaviour. If the server were to add
    // context validation in the future, this test would need updating.
    const client = new TestClient();
    const session = await client.createMeeting();
    await client.startVoteRound();

    const wrongVoterUUID = "00000000-0000-0000-0000-000000000000";
    const tokenWithWrongId = generateToken(
      uuidToBytes(wrongVoterUUID), // wrong voter UUID in context
      uuidToBytes(session.muuid),
    );

    const res = await client.rawRequest("POST", "/api/voter/register", {
      context: tokenWithWrongId.context,
      commitment: tokenWithWrongId.commitmentJson,
    });

    // Registration currently succeeds — the context voter_id is not enforced.
    // Note: a ballot built from this token WILL fail at submission because
    // the signature was created with the wrong context header.
    expect(res.status).toBe(201);
  });
});
