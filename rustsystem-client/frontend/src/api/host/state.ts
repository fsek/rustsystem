export type StartVoteRequest = {
  name: string;
};

export async function StartVote(req: StartVoteRequest) {
  await fetch("api/host/start-vote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
}
