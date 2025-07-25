import React, { useEffect, useState } from 'react';

export const RunInvite: React.FC = () => {
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
  return (
    <div>
      Access Granted!
      <img src={imageUrl} alt={'Could not load QR code'} />

    </div>
  );
}

export default RunInvite;
