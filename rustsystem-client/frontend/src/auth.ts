export async function Auth(muid: any): Promise<boolean> {
  const res = await fetch("api/auth-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ muid: muid }),
  });
  const data = await res.json();
  const obj = JSON.parse(data);
  return obj["success"];
}

// Enum style status check
export const AuthStatus = {
  Loading: 1,
  Granted: 2,
  Denied: 3,
} as const;

export type AuthStatus = (typeof AuthStatus)[keyof typeof AuthStatus];
