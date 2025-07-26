import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Auth, AuthStatus, type AuthRequest } from '@/api/auth';
import Unauthorized from '@/components/error-pages/unauthorized.tsx';
import HostPage from '@/components/meeting/host';
import VoterPage from '@/components/meeting/voter';

export const Route = createFileRoute('/meeting')({
  validateSearch: (search) => {
    return {
      muid: search.muid ?? "",
      uuid: search.uuid ?? "",
    };
  },

  component: RouteComponent,
})

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
          setAuthStatus(AuthStatus.VerifiedNonHost)
        }
      } else {
        setAuthStatus(AuthStatus.Denied);
      }
    });
  }, []);

  var page = undefined;

  if (authStatus === AuthStatus.Loading) page = <div>Authenticating...</div>;
  if (authStatus === AuthStatus.VerifiedHost) page = <HostPage muid={muid} />;
  if (authStatus === AuthStatus.VerifiedNonHost) page = <VoterPage muid={muid} uuid={uuid} />;
  if (authStatus === AuthStatus.Denied) page = <div><Unauthorized /></div>;

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      {page}
    </div>
  );
}
