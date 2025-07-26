import { newVoter, type newVoterRequest } from '@/api/newVoter';
import React, { useEffect, useState } from 'react';

export const RunInvite: React.FC = () => {
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);

  useEffect(() => {
    newVoter({} as newVoterRequest)
      .then(res => {
        const url = URL.createObjectURL(res.blob);
        setImageUrl(url);
      })
      .catch(console.error);
  }, []);
  return (
    <div>
      Access Granted!
      <img src={imageUrl} alt={'Could not load QR code'} />

    </div>
  );
}

export default RunInvite;
