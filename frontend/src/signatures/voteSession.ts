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
  type RegistrationSuccessResponse,
  type GeneratedToken,
  type BallotMetaData,
  type CommitmentJson,
  type ProofContext,
} from "./signatures";

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

// ─── Types ───────────────────────────────────────────────────────────────────

export interface SessionIds {
  uuuid: string; // voter/user UUID
  muuid: string; // meeting UUID
}

// Uint8Arrays are not JSON-serialisable; store as plain number[] instead.
export interface StoredVoteData {
  muuid: string;
  uuuid: string;
  token: number[];
  blindFactor: number[];
  commitmentJson: CommitmentJson;
  context: ProofContext;
  signature: unknown;
  metadata: BallotMetaData;
}

// ─── localStorage persistence ─────────────────────────────────────────────────
//
// Security model:
//   • `token` and `blindFactor` are the voter's one-time anonymous credentials.
//     They must never be sent to the server until vote submission, and must
//     survive page refreshes / browser restarts so the user can still vote.
//
//   • localStorage is readable by any JavaScript on the same origin.
//     An XSS attack could steal the token. Mitigations applied here:
//       1. Auto-clear on successful vote submission — the token is spent anyway.
//       2. Clear when the vote round changes — the old registration is invalid.
//       3. "Clear token" button for shared/public computers.
//     Additionally, the site should enforce a strict Content-Security-Policy
//     header to reduce XSS surface (a server-side concern).
//
//   • Stealing the token lets an attacker submit a vote INSTEAD of the user
//     (bearer-credential misuse), but it does NOT break vote anonymity:
//     the server still cannot link a submitted ballot to any voter identity.
//     That guarantee is structural (blind signature), not secret-dependent.
//
//   • Encrypting the payload at rest (SubtleCrypto + user passphrase) would
//     further reduce XSS risk, but adds significant UX friction and is
//     considered overkill for a first version of this voting system.
//     It should, however, be considered for a future update.

const STORAGE_KEY = "fsek-vote-session";

export function loadVoteData(): StoredVoteData | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as StoredVoteData;
  } catch {
    return null;
  }
}

export function saveVoteData(
  ids: SessionIds,
  token: GeneratedToken,
  reg: RegistrationSuccessResponse,
): void {
  const data: StoredVoteData = {
    muuid: ids.muuid,
    uuuid: ids.uuuid,
    token: Array.from(token.token),
    blindFactor: Array.from(token.blindFactor),
    commitmentJson: token.commitmentJson,
    context: token.context,
    signature: reg.signature,
    metadata: reg.metadata,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
}

export function clearVoteData(): void {
  localStorage.removeItem(STORAGE_KEY);
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
): Promise<SessionIds> {
  const res = await apiFetch("/api/create-meeting", {
    method: "POST",
    body: JSON.stringify({ title, host_name: hostName }),
  });

  const data = await res.json();
  if (!res.ok) throw new Error(`HTTP ${res.status}`);

  // CreateMeetingResponse: { uuuid, muuid }
  // uuuid = voter/user UUID of the host, muuid = meeting UUID
  return { uuuid: data.uuuid, muuid: data.muuid };
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

  if (!res.ok) throw new Error(`HTTP ${res.status}`);
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

  if (!res.ok) throw new Error(`HTTP ${res.status}`);
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
  const data = await res.json();
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return data as TallyResult;
}

/**
 * GET /api/host/get-tally — retrieve the last computed tally without changing state.
 *
 * Throws on non-2xx (e.g. HTTP 410 when no tally has been computed yet).
 */
export async function getTally(): Promise<TallyResult> {
  const res = await apiFetch("/api/host/get-tally", { method: "GET" });
  const data = await res.json();
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return data as TallyResult;
}

// ─── Registration ─────────────────────────────────────────────────────────────

export interface RegisterResult {
  token: GeneratedToken;
  regResponse: RegistrationSuccessResponse;
}

/**
 * Generate a blind-signature token and POST it to /api/voter/register.
 *
 * Throws an `Error` with a message like "HTTP 409" on a non-2xx response.
 * The caller is responsible for persisting the result via `saveVoteData`.
 */
export async function registerVoter(
  session: SessionIds,
): Promise<RegisterResult> {
  const voterBytes = uuidToBytes(session.uuuid);
  const meetingBytes = uuidToBytes(session.muuid);
  const token = generateToken(voterBytes, meetingBytes);

  const body = {
    context: token.context,
    commitment: token.commitmentJson,
  };

  const res = await apiFetch("/api/voter/register", {
    method: "POST",
    body: JSON.stringify(body),
  });

  const data = await res.json();
  if (!res.ok) throw new Error(`HTTP ${res.status}`);

  return { token, regResponse: data as RegistrationSuccessResponse };
}

// ─── Vote submission ──────────────────────────────────────────────────────────

/**
 * Build a padded ballot and POST it to /api/voter/submit.
 *
 * Throws an `Error` with a message like "HTTP 400" on a non-2xx response.
 * The caller is responsible for clearing the stored token after a successful call.
 */
export async function submitVote(
  storedToken: GeneratedToken,
  storedRegResponse: RegistrationSuccessResponse,
  choice: number[] | null,
): Promise<void> {
  const ballot = buildBallot(
    storedRegResponse.metadata,
    choice,
    storedToken.token,
    storedToken.blindFactor,
    storedRegResponse.signature,
  );

  const res = await apiFetch("/api/voter/submit", {
    method: "POST",
    body: JSON.stringify(ballot),
  });

  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}
