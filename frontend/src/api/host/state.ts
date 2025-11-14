import {
  type BallotMetaData,
  start_vote_json_req,
} from "@/pkg/rustsystem_client";
import { withWasm } from "@/utils/wasm";
import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type StartVoteRequest = {
  name: string;
  metadata: BallotMetaData;
};

type StartVoteResponse = {};

export async function StartVote(
  req: StartVoteRequest,
): Promise<Result<StartVoteResponse, APIError>> {
  try {
    const requestBody = await withWasm(() =>
      start_vote_json_req(req.name, req.metadata),
    );

    const res = await fetch("/api/host/start-vote", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify(requestBody),
    });

    if (res.ok) {
      return ok({} as StartVoteResponse);
    } else {
      const obj = await res.json();
      return err(obj as APIError);
    }
  } catch (error) {
    return err({
      code: "WasmError",
      message: "Failed to prepare vote data. Please try again.",
      httpStatus: 500,
      timestamp: new Date().toISOString(),
      endpoint: { method: "POST", path: "/api/host/start-vote" },
    } as APIError);
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
  const res = await fetch("/api/host/tally", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "include",
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
  const res = await fetch("/api/host/end-vote-round", {
    method: "DELETE",
    credentials: "include",
  });

  if (res.ok) {
    return ok({} as TallyResponse);
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
