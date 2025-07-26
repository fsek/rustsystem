export type AuthRequest = {
  muid: any;
};

type AuthResponse = {
  muid?: string;
  uuid?: string;
  is_host?: boolean;
  success: boolean;
};

export async function Auth(req: AuthRequest): Promise<AuthResponse> {
  const res = await fetch("api/auth-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const data = await res.json();
  const obj = JSON.parse(data);
  return obj as AuthResponse;
}

// Enum style status check
export const AuthStatus = {
  Loading: 1,
  VerifiedHost: 2,
  VerifiedNonHost: 3,
  Denied: 4,
} as const;

export type AuthStatus = (typeof AuthStatus)[keyof typeof AuthStatus];
