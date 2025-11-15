import { type Result, ok, err } from "@/result";
import type { APIError } from "@/api/error";

export type VoteProgressRequest = {};

export type VoteProgressResponse = {
  isActive: boolean;
  isTally: boolean;
  totalVotesCast: number;
  totalParticipants: number;
  voteName: string | null;
};

export async function getVoteProgress(
  _req: VoteProgressRequest,
): Promise<Result<VoteProgressResponse, APIError>> {
  const res = await fetch("/api/common/vote-progress", {
    method: "GET",
    credentials: "include",
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as VoteProgressResponse);
  } else {
    return err(obj as APIError);
  }
}

export function voteProgressWatch(): EventSource {
  return new EventSource("/api/common/vote-progress-watch", {
    withCredentials: true,
  });
}
