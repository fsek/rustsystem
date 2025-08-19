import { err, ok, type Result } from "@/result";

export type AuthMeetingRequest = {
  muid: any;
};

type AuthMeetingResponse = {
  muid: string;
  uuid: string;
  is_host: boolean;
};

enum AuthMeetingError {
  InvalidMUID = "InvalidMUID",
  MUIDMismatch = "MUIDMismatch",
}

export async function Auth(
  req: AuthMeetingRequest,
): Promise<Result<AuthMeetingResponse, AuthMeetingError>> {
  const res = await fetch("api/auth-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    const obj = await res.json();
    return ok(obj as AuthMeetingResponse);
  } else {
    const error = await res.json();
    return err(error as AuthMeetingError);
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
