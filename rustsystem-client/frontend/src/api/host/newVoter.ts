import { err, ok, type Result } from "@/result";
import type { APIError } from "../error";

export type startInviteRequest = {};
type startInviteResponse = {};

export async function startInvite(
  _req: startInviteRequest,
): Promise<Result<startInviteResponse, APIError>> {
  const res = await fetch("api/host/start-invite", {
    method: "POST",
    credentials: "include",
  });

  if (res.ok) {
    return ok(res as startInviteResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}

export type newVoterRequest = {};
type newVoterResponse = {
  blob: Blob;
};

export async function newVoter(
  _req: newVoterRequest,
): Promise<Result<newVoterResponse, APIError>> {
  const res = await fetch("api/host/new-voter", {
    method: "POST",
    credentials: "include",
  });

  if (res.ok) {
    return ok({ blob: await res.blob() } as newVoterResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
