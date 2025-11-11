import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type ResetLoginRequest = {
  user_uuuid: string;
};

export async function resetLogin(
  req: ResetLoginRequest,
): Promise<Result<{ qrSvg: string; inviteLink: string }, APIError>> {
  const res = await fetch("/api/host/reset-login", {
    method: "POST",
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    const data = await res.json();
    return ok(data);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
