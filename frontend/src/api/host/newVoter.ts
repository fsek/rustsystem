import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type startInviteRequest = {};
type startInviteResponse = {};

export async function startInvite(
  _req: startInviteRequest,
): Promise<Result<startInviteResponse, APIError>> {
  const res = await fetch("/api/host/start-invite", {
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

export type NewVoterRequest = {
  voterName: string;
  isHost: boolean;
};
type NewVoterResponse = {
  blob: Blob;
};

export async function newVoter(
  req: NewVoterRequest,
): Promise<Result<NewVoterResponse, APIError>> {
  const res = await fetch("/api/host/new-voter", {
    method: "POST",
    credentials: "include",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req),
  });

  if (res.ok) {
    return ok({ blob: await res.blob() } as NewVoterResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
