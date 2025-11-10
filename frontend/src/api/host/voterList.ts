import { type Result, err, ok } from "@/result";
import type { APIError } from "../error";

export type VoterListRequest = {};

export type VoterInfo = {
  name: string;
  uuid: string;
  registered_at: number;
  logged_in: boolean;
  is_host: boolean;
};

export type VoterListResponse = {
  voters: Array<{
    name: string;
    uuid: string;
    registeredAt: string;
    loggedIn: boolean;
    isHost: boolean;
  }>;
};

export async function VoterList(
  _req: VoterListRequest,
): Promise<Result<VoterListResponse, APIError>> {
  const res = await fetch("/api/host/voter-list", {
    method: "GET",
    credentials: "include",
  });

  if (res.ok) {
    const data: VoterInfo[] = await res.json();
    // Backend returns Vec<VoterInfo> - convert to our format
    const voters = data.map((voterInfo) => ({
      name: voterInfo.name,
      uuid: voterInfo.uuid,
      registeredAt: new Date(voterInfo.registered_at * 1000).toLocaleString(),
      loggedIn: voterInfo.logged_in,
      isHost: voterInfo.is_host,
    }));
    return ok({ voters });
  } else {
    const obj = await res.json();
    return err(obj as APIError);
  }
}
