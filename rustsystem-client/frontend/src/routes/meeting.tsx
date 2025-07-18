import { useEffect, useState } from 'react';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { Auth, AuthStatus } from '../auth.ts';
import { Unauthorized } from '../components/unauthorized.tsx';

export const Route = createFileRoute('/meeting')({
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
  
  const navigate = useNavigate();
  function invitePage() {
    navigate({ to: "/invite", search: { muid: muid }});
  }
  
  useEffect(() => {
    Auth(muid).then((res) => {
      if (res) {
        console.log("Successfully logged in");
        setAuthStatus(AuthStatus.Granted);
      } else {
        console.log("Could not log in");
        setAuthStatus(AuthStatus.Denied);
      }
    });  
  }, []);
  
  if (authStatus === AuthStatus.Loading) return <div>Checking...</div>;
  if (authStatus === AuthStatus.Granted) return <div>Access Granted! You can now invite people!<button onClick={invitePage}>Invite!</button></div>;
  if (authStatus === AuthStatus.Denied) return <div><Unauthorized /></div>;
}
