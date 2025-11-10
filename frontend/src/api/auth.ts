import { type Result, err, ok } from "@/result";
import type { APIError } from "./error";

export type AuthMeetingRequest = {
  muuid: string;
};

type AuthMeetingResponse = {
  muuid: string;
  uuid: string;
  is_host: boolean;
};

export async function Auth(
  req: AuthMeetingRequest,
): Promise<Result<AuthMeetingResponse, APIError>> {
  const res = await fetch("/api/auth-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as AuthMeetingResponse);
  } else {
    return err(obj as APIError);
  }
}

// Enum style status check
export const AuthStatus = {
  Loading: 1,
  VerifiedHost: 2,
  VerifiedNonHost: 3,
  Denied: 4,
} as const;

export type AuthStatus = (typeof AuthStatus)[keyof typeof AuthStatus];
