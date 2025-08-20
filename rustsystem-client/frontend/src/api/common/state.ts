import { err, ok, type Result } from "@/result";

export type VoteActiveRequest = {};

type VoteActiveResponse = {
  isActive: boolean;
};

enum VoteActiveError {
  MUIDNotFound = "MUIDNotFound",
}

export async function VoteActive(
  _req: VoteActiveRequest,
): Promise<Result<VoteActiveResponse, VoteActiveError>> {
  const res = await fetch("api/common/vote-active", {
    method: "GET",
    credentials: "include",
  });

  const obj = await res.json();
  if (res.ok) {
    return ok(obj as VoteActiveResponse);
  } else {
    return err(obj as VoteActiveError);
  }
}
