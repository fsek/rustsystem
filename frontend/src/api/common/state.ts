import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type VoteActiveRequest = {};

type VoteActiveResponse = {
  isActive: boolean;
};

export async function VoteActive(
  _req: VoteActiveRequest,
): Promise<Result<VoteActiveResponse, APIError>> {
  const res = await fetch("/api/common/vote-active", {
    method: "GET",
    credentials: "include",
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as VoteActiveResponse);
  } else {
    return err(obj as APIError);
  }
}

export function voteStateWatch(): EventSource {
  return new EventSource("/api/common/vote-state-watch");
}
