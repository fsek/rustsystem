import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Auth, AuthStatus, type AuthMeetingRequest } from '@/api/auth';
import RunInvite from '@/components/invite/run_invite.tsx';
import { matchResult } from '@/result';
import ErrorHandler from '@/components/error';
import type { APIError } from '@/api/error';

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
  const [error, setError] = useState<APIError | null>(null);

  const search = Route.useSearch();

  const muuid = search.muid

  useEffect(() => {
    Auth({ muuid } as AuthMeetingRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.is_host) {
            setAuthStatus(AuthStatus.VerifiedHost);
          } else {
            setAuthStatus(AuthStatus.VerifiedNonHost)
          }
        },
        Err: (err) => {
          setError(err);
        },
      })
    });
  }, []);

  if (error) {
    return (<ErrorHandler error={error} />);
  }
  if (authStatus === AuthStatus.Loading) return <div>Checking...</div>;
  if (authStatus === AuthStatus.VerifiedHost) return <RunInvite />;
}
