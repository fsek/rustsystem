import React, { useEffect, useState, type ReactElement } from 'react';
import VotePage from '@/components/meeting/vote_page';
import { VoteActive, type VoteActiveRequest } from '@/api/common/state';
import { startVoteWait } from '@/api/voter/state';

type VoterPageProps = {
  muid: any,
  uuid: any,
}

const VoterPage: React.FC<VoterPageProps> = ({ muid, uuid }) => {
  const voteEvent = startVoteWait();
  const [voteActive, setVoteActive] = useState<boolean>(false);

  useEffect(() => {
    // Explicitly check for voteActive being true.
    VoteActive({} as VoteActiveRequest).then((res) => {
      if (res.isActive === true) {
        setVoteActive(true);
      } else {
        setVoteActive(false);
      }
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
