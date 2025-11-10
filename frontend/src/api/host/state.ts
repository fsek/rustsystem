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

export type TallyRequest = {};

export type TallyResponse = {
  score: Object;
  blank: number;
};

export async function Tally(
  _req: TallyRequest,
): Promise<Result<TallyResponse, APIError>> {
  const res = await fetch("api/host/tally", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as TallyResponse);
  } else {
    return err(obj as APIError);
  }
}

export type EndVoteRoundRequest = {};
type EndVoteRoundResponse = {};

export async function EndVoteRound(
  _req: EndVoteRoundRequest,
): Promise<Result<EndVoteRoundResponse, APIError>> {
  const res = await fetch("api/host/end-vote-round", {
    method: "DELETE",
  });

  if (res.ok) {
    return ok({} as TallyResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
