import React, { useEffect, useState, type ReactElement } from 'react';
import VotePage from '@/components/meeting/vote_page';

type VoterPageProps = {
  muid: any,
  uuid: any,
}

const VoterPage: React.FC<VoterPageProps> = ({ muid, uuid }) => {
  const voteEvent = new EventSource("/api/events/vote-watch");
  const [voteActive, setVoteActive] = useState<boolean>(false);

  useEffect(() => {
    fetch("api/vote-active", {
      method: "GET",
      credentials: "include",
    }).then((res) => {
      res.json().then((data) => {
        const obj = JSON.parse(data);
        // Explicitly check for voteActive being true.
        if (obj.voteActive === true) {
          setVoteActive(true);
        } else {
          setVoteActive(false);
        }
      });
    });
  }, []);

  voteEvent.onmessage = function (event) {
    if (event.data === "Start") {
      setVoteActive(true);
    } else if (event.data === "Stop") {
      setVoteActive(false);
    }
  }

  if (voteActive) {
    return <VotePage muid={muid} uuid={uuid} />
  } else {
    return WaitPage();
  }
}

function WaitPage(): ReactElement {
  return (
    <div>
      You are a voter in this meeting. Please stand by and wait for the host to start a vote.
    </div>
  );
}

export default VoterPage;
