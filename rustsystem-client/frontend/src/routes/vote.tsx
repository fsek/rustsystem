import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useState } from "react";
import init, { send_vote, register } from "@/pkg/rustsystem_client.js";
import { Auth, AuthStatus, type AuthRequest } from '@/api/auth';

export const Route = createFileRoute("/vote")({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
      uuid: search.uuid ?? "",
    };
  },

  component: RouteComponent,
});

function RouteComponent() {
  const [authStatus, setAuthStatus] = useState<AuthStatus>(AuthStatus.Loading);
  const search = Route.useSearch();
  const muid = search.muid;
  const uuid = search.uuid;

  useEffect(() => {
    Auth({ muid } as AuthRequest).then((res) => {
      if (res.success) {
        if (res.is_host) {
          setAuthStatus(AuthStatus.VerifiedHost);
        } else {
          setAuthStatus(AuthStatus.VerifiedNonHost);
        }
        TestVote(uuid, muid);
      } else {
        setAuthStatus(AuthStatus.Denied);
      }
    });
  }, []);

  return <div>Testing Vote API, authStatus is {authStatus}</div>
}

async function TestVote(uuid: any, muid: any) {
  await init();

  const ballot = await register(muid, uuid);
  console.log(ballot.signature());
  console.log(ballot.proof());

  const response = await send_vote(ballot);
  console.log(response);
}
