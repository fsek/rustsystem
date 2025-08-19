import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Auth, AuthStatus, type AuthMeetingRequest } from '@/api/auth';
import Unauthorized from '@/components/error-pages/unauthorized.tsx';
import RunInvite from '@/components/invite/run_invite.tsx';
import { matchResult } from '@/result';

export const Route = createFileRoute('/invite')({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
    };
  },

  component: RouteComponent,
})

function RouteComponent() {
  const [authStatus, setAuthStatus] = useState<AuthStatus>(AuthStatus.Loading);

  const search = Route.useSearch();

  const muid = search.muid

  useEffect(() => {
    Auth({ muid } as AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.is_host) {
            setAuthStatus(AuthStatus.VerifiedHost);
          } else {
            setAuthStatus(AuthStatus.VerifiedNonHost)
          }
        },
        Err: (err) => {
          setAuthStatus(AuthStatus.Denied);
          console.error(err)
        },
      })
    });
  }, []);



  if (authStatus === AuthStatus.Loading) return <div>Checking...</div>;
  if (authStatus === AuthStatus.VerifiedHost) return <RunInvite />;
  if (authStatus === AuthStatus.Denied) return <div><Unauthorized /></div>;
}
