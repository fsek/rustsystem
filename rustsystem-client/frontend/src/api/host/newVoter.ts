export type newVoterRequest = {};
type newVoterResponse = {
  blob: Blob;
};

export async function newVoter(
  _req: newVoterRequest,
): Promise<newVoterResponse> {
  const res = await fetch("api/host/new-voter", {
    method: "POST",
    credentials: "include",
  });

  return { blob: await res.blob() } as newVoterResponse;
}

export type startInviteRequest = {};
type startInviteResponse = {};
export async function startInvite(
  _req: startInviteRequest,
): Promise<startInviteResponse> {
  const res = await fetch("api/host/start-invite", {
    method: "POST",
    credentials: "include",
  });

  return res as startInviteResponse;
}
