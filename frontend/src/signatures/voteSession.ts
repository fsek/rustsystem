/**
 * Voter session management: localStorage persistence, registration, and vote submission.
 *
 * These functions encapsulate the full voter workflow so they can be reused
 * across different UI pages (e.g. signature-dev and the production voting page).
 */

import {
  generateToken,
  buildBallot,
  uuidToBytes,
  type BallotMetaData,
} from "./signatures";
import { handleErrorResponse } from "@/api/error";

// ─── API fetch ────────────────────────────────────────────────────────────────
// Set VITE_API_ENDPOINT at build time (e.g. https://server.fsek.studentorg.lu.se).
// Defaults to "" so that relative /api/... paths are used in development (Vite proxy).

export const API_BASE = (import.meta.env.VITE_API_ENDPOINT ?? "").replace(
  /\/$/,
  "",
);

export function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

export async function apiFetch(
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  return fetch(apiUrl(path), {
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    ...init,
  });
}

// ─── TrustAuth fetch ──────────────────────────────────────────────────────────
// Set API_ENDPOINT_TRUSTAUTH at build time to the trustauth origin visible from the browser
// (e.g. http://localhost:2443 in dev, https://trustauth.fsektionen.se in prod).
// Must use the same hostname as the server so that SameSite=Strict cookies are sent correctly.

export const TRUSTAUTH_BASE = (
  import.meta.env.API_ENDPOINT_TRUSTAUTH ?? ""
).replace(/\/$/, "");

export function trustAuthUrl(path: string): string {
  return `${TRUSTAUTH_BASE}${path}`;
}

export async function trustAuthFetch(
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  return fetch(trustAuthUrl(path), {
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    ...init,
  });
}

export async function trustAuthLogin(
  uuuid: string,
  muuid: string,
): Promise<void> {
  const res = await trustAuthFetch("/api/login", {
    method: "POST",
    body: JSON.stringify({ uuuid, muuid }),
  });
  if (!res.ok) await handleErrorResponse(res);
}

// ─── Types ───────────────────────────────────────────────────────────────────

export interface SessionIds {
  uuuid: string; // voter/user UUID
  muuid: string; // meeting UUID
}

export async function getSessionIds(): Promise<SessionIds> {
  const res = await apiFetch("/api/session-ids");
  const { uuuid, muuid } = await res.json();

  return { uuuid, muuid };
}

// ─── Meeting & vote-round management ─────────────────────────────────────────

/**
 * POST /api/create-meeting — create a new meeting and log in as host.
 *
 * Returns the `SessionIds` for all subsequent calls.
 * Throws an `Error` with a message like "HTTP 500" on a non-2xx response.
 */
export async function createMeeting(
  title: string,
  hostName: string,
  pubKey: string,
): Promise<SessionIds> {
  const res = await apiFetch("/api/create-meeting", {
    method: "POST",
    body: JSON.stringify({ title, host_name: hostName, pub_key: pubKey }),
  });

  if (!res.ok) await handleErrorResponse(res);
  const data = await res.json();

  // CreateMeetingResponse: { uuuid, muuid }
  // uuuid = voter/user UUID of the host, muuid = meeting UUID
  const ids: SessionIds = { uuuid: data.uuuid, muuid: data.muuid };
  await trustAuthLogin(ids.uuuid, ids.muuid);
  return ids;
}

/**
 * POST /api/host/start-vote — open a new vote round.
 *
 * Throws an `Error` with a message like "HTTP 409" on a non-2xx response.
 */
export async function startVoteRound(
  name: string,
  shuffle: boolean,
  metadata: BallotMetaData,
): Promise<void> {
  const res = await apiFetch("/api/host/start-vote", {
    method: "POST",
    body: JSON.stringify({ name, shuffle, metadata }),
  });

  if (!res.ok) await handleErrorResponse(res);
}

/**
 * DELETE /api/host/end-vote-round — close the active vote round.
 *
 * Throws an `Error` with a message like "HTTP 404" on a non-2xx response.
 */
export async function endVoteRound(): Promise<void> {
  const res = await apiFetch("/api/host/end-vote-round", {
    method: "DELETE",
  });

  if (!res.ok) await handleErrorResponse(res);
}

// ─── Tally ────────────────────────────────────────────────────────────────────

/** Score map from candidate name → vote count, plus a blank-vote count. */
export interface TallyResult {
  score: Record<string, number>;
  blank: number;
}

/**
 * POST /api/host/tally — close the active vote round and compute results.
 *
 * Transitions the server from Voting → Tally state. Throws on non-2xx.
 */
export async function tally(): Promise<TallyResult> {
  const res = await apiFetch("/api/host/tally", { method: "POST" });
  if (!res.ok) await handleErrorResponse(res);
  return res.json() as Promise<TallyResult>;
}

/**
 * GET /api/host/get-tally — retrieve the last computed tally without changing state.
 *
 * Throws on non-2xx (e.g. HTTP 410 when no tally has been computed yet).
 */
export async function getTally(): Promise<TallyResult> {
  const res = await apiFetch("/api/host/get-tally", { method: "GET" });
  if (!res.ok) await handleErrorResponse(res);
  return res.json() as Promise<TallyResult>;
}

// ─── Voter status queries ─────────────────────────────────────────────────────

/**
 * GET /api/voter/is-registered — returns true if the current voter (identified
 * by their JWT) has already registered for the active vote round.
 */
export async function isRegistered(): Promise<boolean> {
  const res = await trustAuthFetch("/api/is-registered");
  if (!res.ok) await handleErrorResponse(res);
  const body = await res.json();
  return body["isRegistered"];
}

/**
 * POST /api/voter/is-submitted — returns true if the given blind signature has
 * already been spent (i.e. the vote has gone through).
 */
export async function isSubmitted(signature: unknown): Promise<boolean> {
  const res = await apiFetch("/api/voter/is-submitted", {
    method: "POST",
    body: JSON.stringify({ signature }),
  });
  if (!res.ok) await handleErrorResponse(res);
  return res.json();
}

// ─── Registration ─────────────────────────────────────────────────────────────

/**
 * Generate a blind-signature token and POST it to trustauth /api/register.
 * The token, blind factor, commitment, context, and resulting signature are
 * stored server-side on trustauth. Nothing is persisted client-side.
 *
 * Throws an `Error` with a message like "HTTP 409" on a non-2xx response.
 */
export async function registerVoter(session: SessionIds): Promise<void> {
  const voterBytes = uuidToBytes(session.uuuid);
  const meetingBytes = uuidToBytes(session.muuid);
  const token = generateToken(voterBytes, meetingBytes);

  const body = {
    context: token.context,
    commitment: token.commitmentJson,
    token: Array.from(token.token),
    blind_factor: Array.from(token.blindFactor),
  };

  const res = await trustAuthFetch("/api/register", {
    method: "POST",
    body: JSON.stringify(body),
  });

  if (!res.ok) await handleErrorResponse(res);
}

// ─── Vote data retrieval ───────────────────────────────────────────────────────

export interface VoteData {
  token: number[];
  blind_factor: number[];
  signature: unknown;
}

/**
 * GET /vote-data — retrieve the stored token, blind factor, and signature from
 * trustauth. Called just before vote submission.
 *
 * Throws an `Error` with a message like "HTTP 404" if not registered.
 */
export async function getVoteData(): Promise<VoteData> {
  const res = await trustAuthFetch("/api/vote-data");
  if (!res.ok) await handleErrorResponse(res);
  return res.json() as Promise<VoteData>;
}

// ─── Vote submission ──────────────────────────────────────────────────────────

/**
 * Build a padded ballot and POST it to /api/voter/submit.
 *
 * Throws an `Error` with a message like "HTTP 400" on a non-2xx response.
 */
export async function submitVote(
  voteData: VoteData,
  metadata: BallotMetaData,
  choice: number[] | null,
): Promise<void> {
  const ballot = buildBallot(
    metadata,
    choice,
    new Uint8Array(voteData.token),
    new Uint8Array(voteData.blind_factor),
    voteData.signature,
  );

  const res = await apiFetch("/api/voter/submit", {
    method: "POST",
    body: JSON.stringify(ballot),
  });

  if (!res.ok) await handleErrorResponse(res);
}
