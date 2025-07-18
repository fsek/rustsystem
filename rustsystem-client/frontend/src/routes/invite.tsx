import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Auth, AuthStatus } from '../auth.ts';
import { Unauthorized } from '../components/unauthorized.tsx';

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
      if (res) {
        setAuthStatus(AuthStatus.Granted);
      } else {
        setAuthStatus(AuthStatus.Denied);
      }
    });  
  }, []);

  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);

  useEffect(() => {
    fetch("api/new-voter", {
      method: "POST",
      credentials: "include",
    }).then(res => res.blob())
      .then(blob => {
        const url = URL.createObjectURL(blob);
        setImageUrl(url);
      })
      .catch(console.error);
  }, []);

  if (authStatus === AuthStatus.Loading) return <div>Checking...</div>;
  if (authStatus === AuthStatus.Granted) return <div>Access Granted!<img src={imageUrl} alt={'Could not load QR code'} /></div>;
  if (authStatus === AuthStatus.Denied) return <div><Unauthorized /></div>;
}
