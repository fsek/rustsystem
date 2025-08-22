import { err, ok, type Result } from "@/result";

export type startInviteRequest = {};
type startInviteResponse = {};
enum StartInviteError {
  MUIDNotFound = "MUIDNotFound",
}
export async function startInvite(
  _req: startInviteRequest,
): Promise<Result<startInviteResponse, StartInviteError>> {
  const res = await fetch("api/host/start-invite", {
    method: "POST",
    credentials: "include",
  });

  if (res.ok) {
    return ok(res as startInviteResponse);
  } else {
    const obj = await res.json();
    return err(obj as StartInviteError);
  }
}

export type newVoterRequest = {};
type newVoterResponse = {
  blob: Blob;
};

enum newVoterError {
  MUIDNotFound = "MUIDNotFound",
}

export async function newVoter(
  _req: newVoterRequest,
): Promise<Result<newVoterResponse, newVoterError>> {
  const res = await fetch("api/host/new-voter", {
    method: "POST",
    credentials: "include",
  });

  if (res.ok) {
    return ok({ blob: await res.blob() } as newVoterResponse);
  } else {
    const obj = await res.json();
    return err(obj as newVoterError);
  }
}
