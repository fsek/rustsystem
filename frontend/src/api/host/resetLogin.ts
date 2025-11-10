import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type ResetLoginRequest = {
  user_uuuid: string;
};

export async function resetLogin(
  req: ResetLoginRequest,
): Promise<Result<{ blob: Blob }, APIError>> {
  const res = await fetch("/api/host/reset-login", {
    method: "POST",
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    const blob = await res.blob();
    return ok({ blob });
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
