import { BallotMetaData, start_vote_json_req } from "@/pkg/rustsystem_client";
import { err, ok, type Result } from "@/result";
import type { APIError } from "../error";

export type StartVoteRequest = {
  name: string;
  metadata: BallotMetaData;
};

type StartVoteResponse = {};

export async function StartVote(
  req: StartVoteRequest,
): Promise<Result<StartVoteResponse, APIError>> {
  const res = await fetch("api/host/start-vote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(start_vote_json_req(req.name, req.metadata)),
  });

  if (res.ok) {
    return ok({} as StartVoteResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
