/**
 * Casting a vote (`docs/PROTOCOL.md` §5.2–5.3, §6).
 *
 * 1. Build the ballot `{"v":1,"round":…,"choice":…,"nonce":…}` and check it against the same
 *    rules the server uses (`rustsystem-server/src/ballot.rs`). A ballot trustauth has signed
 *    but the server rejects would cost the voter their vote, so this check must come first.
 * 2. Blind it with the round's public key — taken from the *server* — and have trustauth sign
 *    the blinded value. Trustauth sees who is voting, but not what.
 * 3. Unblind. This also verifies the signature against the server's key, so trustauth can't
 *    sign with a voter-specific key to recognise the ballot later.
 * 4. Submit `prepared + sig` to the server with no cookies, retrying until it's received.
 *
 * Nothing is ever stored: the ballot exists only in this function's memory. After a refresh,
 * {@link voteStatus} tells the page whether this voter has voted in the current round.
 */

import { RSABSSA } from "@cloudflare/blindrsa-ts";

import { server, trustauth } from "@/api/client";
import { fromBase64Url, toBase64Url } from "@/api/encoding";
import { ApiError, isApiError } from "@/api/error";

const suite = RSABSSA.SHA384.PSS.Randomized();

export interface Round {
  id: string;
  candidates: string[];
  maxChoices: number;
  /** SPKI DER, base64url, from `GET /api/meeting`. */
  publicKey: string;
}

/** `null` is a blank vote. */
export type Choice = number[] | null;

/**
 * Returns why `choice` isn't allowed, or `null` if it is. A choice is a non-empty list of
 * distinct candidate indices, at most `maxChoices` long; a blank vote is `null`.
 */
export function choiceProblem(
  choice: Choice,
  candidates: number,
  maxChoices: number,
): string | null {
  if (choice === null) return null;
  if (choice.length === 0) return "Choose at least one option, or vote blank.";
  if (choice.length > maxChoices)
    return `Choose at most ${maxChoices} options.`;
  if (new Set(choice).size !== choice.length)
    return "Each option can only be chosen once.";
  if (choice.some((c) => !Number.isInteger(c) || c < 0 || c >= candidates)) {
    return "That option doesn't exist.";
  }
  return null;
}

/** The exact bytes that get signed. Choices are sorted, as the server requires. */
export function ballotMessage(roundId: string, choice: Choice): Uint8Array {
  const nonce = toBase64Url(crypto.getRandomValues(new Uint8Array(32)));
  const sorted = choice === null ? null : [...choice].sort((a, b) => a - b);
  const json = JSON.stringify({ v: 1, round: roundId, choice: sorted, nonce });
  return new TextEncoder().encode(json);
}

export async function importRoundKey(publicKeyB64: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "spki",
    fromBase64Url(publicKeyB64),
    { name: "RSA-PSS", hash: "SHA-384" },
    true,
    ["verify"],
  );
}

/** Steps 1–3: returns a signed ballot ready to submit. Uses up this voter's one signature. */
export async function signBallot(
  round: Round,
  choice: Choice,
): Promise<{ prepared: Uint8Array; sig: Uint8Array }> {
  const problem = choiceProblem(
    choice,
    round.candidates.length,
    round.maxChoices,
  );
  if (problem) throw new ApiError("InvalidBallot", problem, 0);

  const publicKey = await importRoundKey(round.publicKey);
  const prepared = suite.prepare(ballotMessage(round.id, choice));
  const { blindedMsg, inv } = await suite.blind(publicKey, prepared);

  const { blind_sig } = await trustauth.post<{ blind_sig: string }>(
    "/api/sign",
    {
      round: round.id,
      blinded: toBase64Url(blindedMsg),
    },
  );

  let sig: Uint8Array;
  try {
    sig = await suite.finalize(
      publicKey,
      prepared,
      fromBase64Url(blind_sig),
      inv,
    );
  } catch {
    throw new ApiError(
      "InvalidSignature",
      "The signing service returned an invalid signature. Please tell a host.",
      0,
    );
  }
  return { prepared, sig };
}

const RETRY_DELAYS_MS = [
  500, 1000, 2000, 4000, 8000, 15000, 30000, 30000, 30000,
];

function retryable(err: unknown): boolean {
  return (
    isApiError(err, "NetworkError", "RateLimited", "Internal") ||
    (err instanceof ApiError && err.status >= 500)
  );
}

/**
 * Step 4: submits anonymously. Safe to repeat — the server counts an identical ballot once and
 * answers `AlreadyReceived`, which means it was counted.
 */
export async function submitBallot(
  meeting: string,
  ballot: { prepared: Uint8Array; sig: Uint8Array },
  sleep: (ms: number) => Promise<void> = (ms) =>
    new Promise((r) => setTimeout(r, ms)),
): Promise<void> {
  const body = {
    meeting,
    prepared: toBase64Url(ballot.prepared),
    sig: toBase64Url(ballot.sig),
  };
  for (let attempt = 0; ; attempt++) {
    try {
      await server.postAnonymous("/api/ballot", body);
      return;
    } catch (err) {
      if (isApiError(err, "AlreadyReceived")) return;
      if (!retryable(err) || attempt >= RETRY_DELAYS_MS.length) throw err;
      await sleep(RETRY_DELAYS_MS[attempt]);
    }
  }
}

/** Signs and submits in one go. */
export async function castVote(
  meeting: string,
  round: Round,
  choice: Choice,
): Promise<void> {
  const ballot = await signBallot(round, choice);
  await submitBallot(meeting, ballot);
}

/** Whether this voter has voted in the current round, from their trustauth login. */
export function voteStatus(): Promise<{
  round: string | null;
  signed: boolean;
}> {
  return trustauth.get("/api/status");
}
