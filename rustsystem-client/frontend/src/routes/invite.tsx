import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Auth, AuthStatus } from '../auth.ts';
import { Unauthorized } from '../components/error-pages/unauthorized.tsx';
import { RunInvite } from '../components/invite/run_invite.tsx';

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
    Auth(muid).then((res) => {
      if (res.success) {
        if (res.is_host) {
          setAuthStatus(AuthStatus.VerifiedHost);
        } else {
          setAuthStatus(AuthStatus.VerifiedNonHost);
        }
      } else {
        setAuthStatus(AuthStatus.Denied);
      }
    });
  }, []);



  if (authStatus === AuthStatus.Loading) return <div>Checking...</div>;
  if (authStatus === AuthStatus.VerifiedHost) return <RunInvite />;
  if (authStatus === AuthStatus.Denied) return <div><Unauthorized /></div>;
}
