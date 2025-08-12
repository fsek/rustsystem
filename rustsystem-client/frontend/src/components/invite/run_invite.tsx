import { startInviteWait } from '@/api/host/inviteEvent';
import { newVoter, startInvite, type newVoterRequest, type startInviteRequest } from '@/api/host/newVoter';
import React, { useEffect, useState } from 'react';

export const RunInvite: React.FC = () => {
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);

  async function connectStart() {
    return new Promise((resolve, reject) => {
      const inviteEvent = startInviteWait();
      inviteEvent.onmessage = function (event) {
        if (event.data == "Ready") {
          get_qr_url().then((url) => {
            setImageUrl(url);
          });
        }
      }

      inviteEvent.onerror = function (err) {
        reject(err);
      }

      inviteEvent.onopen = function () {
        resolve(inviteEvent);
        startInvite({} as startInviteRequest);
      }

    })
  }

  useEffect(() => {
    connectStart()
  }, []);

  return (
    <div>
      Access Granted!
      <img src={imageUrl} alt={'Could not load QR code'} />

    </div>
  );
}

async function get_qr_url(): Promise<string> {
  const res = await newVoter({} as newVoterRequest);
  return URL.createObjectURL(res.blob);
}

export default RunInvite;
