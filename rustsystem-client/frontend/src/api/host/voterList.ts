import { err, ok, type Result } from "@/result";
import type { APIError } from "../error";

export type VoterListRequest = {};

export type VoterListResponse = {
  voters: Array<{
    name: string;
    uuid: string;
  }>;
};

export async function VoterList(
  _req: VoterListRequest,
): Promise<Result<VoterListResponse, APIError>> {
  const res = await fetch("api/host/voter-list", {
    method: "GET",
    credentials: "include",
  });

  if (res.ok) {
    const data = await res.json();
    // Backend returns Vec<(String, String)> - convert to our format
    const voters = data.map(([name, uuid]: [string, string]) => ({
      name,
      uuid,
    }));
    return ok({ voters });
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
