export type VoteActiveRequest = {};
type VoteActiveResponse = {
  isActive: boolean;
};

export async function VoteActive(
  _req: VoteActiveRequest,
): Promise<VoteActiveResponse> {
  const res = await fetch("api/common/vote-active", {
    method: "GET",
    credentials: "include",
  });

  const data = await res.json();
  const obj = JSON.parse(data);
  return obj as VoteActiveResponse;
}
