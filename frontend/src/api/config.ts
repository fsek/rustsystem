import { apiFetch } from "@/signatures/voteSession";

export interface Limits {
  maxNameLength: number;
  maxLabelLength: number;
}

export async function fetchLimits(): Promise<Limits> {
  const res = await apiFetch("/api/limits");
  if (!res.ok) {
    throw new Error(`Failed to fetch limits: ${res.status}`);
  }
  return res.json() as Promise<Limits>;
}
