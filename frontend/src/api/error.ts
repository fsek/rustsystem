/**
 * Error codes the backend can return. Mirrors `ErrorCode` in
 * `rustsystem-core/src/error.rs` — keep the two in sync.
 *
 * `NetworkError` is client-side only: the request never got a response.
 */
export type ErrorCode =
  | "InvalidInput"
  | "BodyTooLarge"
  | "NotLoggedIn"
  | "SessionExpired"
  | "NotHost"
  | "InviteInvalid"
  | "TicketInvalid"
  | "NotFound"
  | "MeetingNotFound"
  | "VoterNotFound"
  | "NameTaken"
  | "CannotRemoveSelf"
  | "RoundInProgress"
  | "VotingClosed"
  | "WrongRound"
  | "NotEligible"
  | "AlreadySigned"
  | "MalformedBallot"
  | "InvalidSignature"
  | "InvalidBallot"
  | "AlreadyReceived"
  | "BallotLimitReached"
  | "RateLimited"
  | "TooManyConnections"
  | "TrustauthUnavailable"
  | "Internal"
  | "NetworkError";

/** An error from the backend, carrying its code and human-readable message. */
export class ApiError extends Error {
  readonly code: ErrorCode;
  readonly status: number;

  constructor(code: ErrorCode, message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
  }
}

export function isApiError(
  err: unknown,
  ...codes: ErrorCode[]
): err is ApiError {
  return (
    err instanceof ApiError && (codes.length === 0 || codes.includes(err.code))
  );
}

/** The user is no longer (or never was) logged in to the service that answered. */
export function isLoggedOut(err: unknown): boolean {
  return isApiError(err, "NotLoggedIn", "SessionExpired");
}

/** A message suitable for showing to the user. */
export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}
