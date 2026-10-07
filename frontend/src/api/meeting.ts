/**
 * Meetings and sessions (`docs/PROTOCOL.md` §4): create, log in, read state, live updates.
 */

import { apiUrl, server, trustauth } from "./client";
import { isLoggedOut } from "./error";

export type Phase = "idle" | "voting" | "tallied";

export interface LoggedIn {
  meeting: string;
  voter: string;
  isHost: boolean;
  ticket: string;
}

export interface Session {
  meeting: string;
  voter: string;
  name: string;
  isHost: boolean;
}

export interface RoundView {
  id: string;
  name: string;
  candidates: string[];
  maxChoices: number;
  /** SPKI DER, base64url. Only present while voting. */
  publicKey: string | null;
  eligible: number;
  received: number;
}

/** One heading of the agenda (`docs/PROTOCOL.md` §4.5). */
export interface AgendaPoint {
  /** 1 for `#`, 2 for `##`, and so on. */
  level: number;
  title: string;
  /** The Markdown under the heading. Shown as plain text, never as HTML. */
  body: string;
}

export interface AgendaView {
  points: AgendaPoint[];
  /** Index into `points`. */
  current: number;
}

export interface MeetingView {
  title: string;
  /** Change counters, the same as the event stream sends. */
  version: number;
  roundVersion: number;
  agendaVersion: number;
  participants: number;
  phase: Phase;
  round: RoundView | null;
  agenda: AgendaView | null;
}

/** The public half of the tally key, derived from the meeting password in the browser. */
export interface TallyKeyBody {
  publicKey: string;
  salt: string;
  tCost: number;
  mCostKib: number;
  pCost: number;
}

export const getSession = () => server.get<Session>("/api/session");
export const getMeeting = () => server.get<MeetingView>("/api/meeting");

/** Exchanges a server-issued ticket for a trustauth session. */
async function trustauthLogin(ticket: string): Promise<void> {
  await trustauth.post("/api/login", { ticket });
}

export async function createMeeting(
  title: string,
  hostName: string,
  tallyKey: TallyKeyBody,
): Promise<LoggedIn> {
  const res = await server.post<LoggedIn>("/api/meetings", {
    title,
    hostName,
    tallyKey,
  });
  await trustauthLogin(res.ticket);
  return res;
}

/** Logs in with an invite link's parameters, to the server and then to trustauth. */
export async function login(
  meeting: string,
  invite: string,
): Promise<LoggedIn> {
  const res = await server.post<LoggedIn>("/api/login", { meeting, invite });
  await trustauthLogin(res.ticket);
  return res;
}

/**
 * Makes sure the trustauth session exists. If the page died between the two logins, the
 * server session is used to get a fresh ticket.
 */
export async function ensureTrustauthSession(): Promise<void> {
  try {
    await trustauth.get("/api/status");
  } catch (err) {
    if (!isLoggedOut(err)) throw err;
    const { ticket } = await server.post<{ ticket: string }>(
      "/api/trustauth-ticket",
    );
    await trustauthLogin(ticket);
  }
}

/**
 * `round` moves when a round opens, closes or resets; `agenda` when the agenda or its current
 * point changes; `version` moves on every change.
 */
export interface Versions {
  version: number;
  round: number;
  agenda: number;
}

/**
 * Calls `onEvent` with the meeting's change counters, once on connect and then on every change.
 * Events carry no state: the caller refetches what it shows. Voter pages need to refetch only
 * when `round` or `agenda` moves; host pages whenever `version` moves. On a dropped connection `onEvent` is
 * called with `null` (refetch to notice a closed meeting); `EventSource` then reconnects itself.
 * Returns a function that stops listening.
 */
export function watchMeeting(
  onEvent: (versions: Versions | null) => void,
): () => void {
  const es = new EventSource(apiUrl("/api/meeting/events"), {
    withCredentials: true,
  });
  es.onmessage = (e) => {
    try {
      onEvent(JSON.parse(e.data) as Versions);
    } catch {
      onEvent(null);
    }
  };
  es.onerror = () => onEvent(null);
  return () => es.close();
}
