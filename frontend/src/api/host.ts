/**
 * Host-only API: the voter list, vote rounds, tally files, closing the meeting.
 */

import { server } from "./client";
import type { RoundView } from "./meeting";

export interface VoterInfo {
  id: string;
  name: string;
  isHost: boolean;
  loggedIn: boolean;
  /** Unix seconds. */
  addedAt: number;
}

export interface Invite {
  voter: string;
  inviteLink: string;
  /** An SVG `data:` URI. */
  qrSvg: string;
}

export interface Counts {
  eligible: number;
  /** How many voters trustauth has signed; `null` if it couldn't be reached. */
  signed: number | null;
  received: number;
}

/** Results as the server sends them: `score[i]` belongs to `round.candidates[i]`. */
export interface ServerTally {
  score: number[];
  blank: number;
}

export interface HostRoundView {
  phase: "idle" | "voting" | "tallied";
  round: RoundView | null;
  counts: Counts | null;
  tally: ServerTally | null;
}

/** Results keyed by candidate name, the shape the result views and exports use. */
export interface TallyResult {
  score: Record<string, number>;
  blank: number;
}

export function toTallyResult(
  candidates: string[],
  tally: ServerTally,
): TallyResult {
  const score: Record<string, number> = {};
  candidates.forEach((c, i) => {
    score[c] = tally.score[i] ?? 0;
  });
  return { score, blank: tally.blank };
}

export interface TallyFileEntry {
  filename: string;
  /** The encrypted file, standard base64. */
  data: string;
}

// ── Voters ───────────────────────────────────────────────────────────────────

export const listVoters = () => server.get<VoterInfo[]>("/api/host/voters");
export const addVoter = (name: string, isHost: boolean) =>
  server.post<Invite>("/api/host/voters", { name, isHost });
export const resetInvite = (voter: string) =>
  server.post<Invite>(`/api/host/voters/${voter}/reset-invite`);
export const removeVoter = (voter: string) =>
  server.delete<void>(`/api/host/voters/${voter}`);
/** Removes every voter who isn't a host. */
export const removeAllVoters = () => server.delete<void>("/api/host/voters");

// ── Rounds ───────────────────────────────────────────────────────────────────

export const getRound = () => server.get<HostRoundView>("/api/host/round");
export const startRound = (
  name: string,
  candidates: string[],
  maxChoices: number,
  shuffle: boolean,
) =>
  server.post<HostRoundView>("/api/host/round", {
    name,
    candidates,
    maxChoices,
    shuffle,
  });
export const closeRound = () =>
  server.post<HostRoundView>("/api/host/round/close");
/** Cancels an open round or clears a closed one's result. */
export const resetRound = () => server.delete<void>("/api/host/round");

// ── Meeting ──────────────────────────────────────────────────────────────────

export const getTallyFiles = () =>
  server.get<TallyFileEntry[]>("/api/host/tally-files");
export const closeMeeting = () => server.delete<void>("/api/host/meeting");
