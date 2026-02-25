/**
 * Host-specific API helpers for the admin panel.
 */

import { apiFetch } from "@/signatures/voteSession";
import type { BallotMetaData } from "@/signatures/signatures";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface VoterInfo {
  name: string;
  uuid: string;
  registered_at: number;
  logged_in: boolean;
  is_host: boolean;
}

export interface NewVoterResponse {
  qrSvg: string;
  inviteLink: string;
}

export interface VoteProgress {
  isActive: boolean;
  isTally: boolean;
  totalVotesCast: number;
  totalParticipants: number;
  voteName: string | null;
  metadata: BallotMetaData | null;
}

// ─── Voter management ─────────────────────────────────────────────────────────

export async function fetchVoterList(): Promise<VoterInfo[]> {
  const res = await apiFetch("/api/host/voter-list");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function addVoter(
  name: string,
  isHost: boolean,
): Promise<NewVoterResponse> {
  const res = await apiFetch("/api/host/new-voter", {
    method: "POST",
    body: JSON.stringify({ voterName: name, isHost: isHost }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function removeVoter(voterUuuid: string): Promise<void> {
  const res = await apiFetch("/api/host/remove-voter", {
    method: "DELETE",
    body: JSON.stringify({ voter_uuuid: voterUuuid }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

export async function removeAllVoters(): Promise<void> {
  const res = await apiFetch("/api/host/remove-all", { method: "DELETE" });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

// ─── Tally files ──────────────────────────────────────────────────────────────

export interface TallyFileEntry {
  filename: string;
  data: string; // base64-encoded encrypted bytes
}

export async function getAllTallyFiles(): Promise<TallyFileEntry[]> {
  const res = await apiFetch("/api/host/get-all-tally");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

// ─── Meeting lifecycle ────────────────────────────────────────────────────────

export async function closeMeeting(): Promise<void> {
  const res = await apiFetch("/api/host/close-meeting", { method: "DELETE" });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

// ─── Vote progress ────────────────────────────────────────────────────────────

export async function fetchVoteProgress(): Promise<VoteProgress> {
  const res = await apiFetch("/api/common/vote-progress");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
