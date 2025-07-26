export type newVoterRequest = {};
type newVoterResponse = {
  blob: Blob;
};

export async function newVoter(
  _req: newVoterRequest,
): Promise<newVoterResponse> {
  const res = await fetch("api/new-voter", {
    method: "POST",
    credentials: "include",
  });

  return { blob: await res.blob() } as newVoterResponse;
}
