import { BallotMetaData, start_vote_json_req } from "@/pkg/rustsystem_client";

export type StartVoteRequest = {
  name: string;
  metadata: BallotMetaData;
};

export async function StartVote(req: StartVoteRequest) {
  await fetch("api/host/start-vote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(start_vote_json_req(req.name, req.metadata)),
  });
}
