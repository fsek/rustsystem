import React, { useEffect, useState, type ReactElement } from 'react';
import RegisterPage from '@/components/meeting/vote-page/register';
import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from '@/components/templates/main';
import { VoteActive, type VoteActiveRequest } from '@/api/common/state';
import { startVoteWait } from '@/api/voter/state';
import DichotomousPage from './vote-page/dichotomous';
import { matchResult } from '@/result';
import type { APIError } from '@/api/error';
import ErrorHandler from '../error';

type VoterPageProps = {
  muid: any,
  uuid: any,
}

export const VotePageDisplay = {
  // Error pages. Something went wrong.
  RegistrationFail: 1,
  VoteFail: 2,

  // Non-error info pages
  VoteFinished: 3,
  AlreadyVoted: 4,

  // Main functional pages
  Wait: 5,
  Register: 6,
  // Vote method pages
  Dichotomous: 7,
} as const;
export type VotePageDisplay = (typeof VotePageDisplay)[keyof typeof VotePageDisplay];

const VoterPage: React.FC<VoterPageProps> = ({ muid, uuid }) => {
  const voteEvent = startVoteWait();
  const [currentVotePageDisplay, setVotePageDisplay] = useState<VotePageDisplay>(VotePageDisplay.Wait);
  const [error, setError] = useState<APIError | null>(null);

  useEffect(() => {
    // Explicitly check for voteActive being true.
    VoteActive({} as VoteActiveRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.isActive === true) {
            setVotePageDisplay(VotePageDisplay.Register);
          } else {
            setVotePageDisplay(VotePageDisplay.Wait);
          }
        },
        Err: (err) => {
          setError(err);
        }
      })
    });
  }, []);

  voteEvent.onmessage = function (event) {
    if (currentVotePageDisplay === VotePageDisplay.Wait)
      if (event.data === "Start") {
        setVotePageDisplay(VotePageDisplay.Register);
      } else if (event.data === "Stop") {
        setVotePageDisplay(VotePageDisplay.Wait);
      }
  }

  if (error) {
    return <ErrorHandler error={error} />
  }
  switch (currentVotePageDisplay) {
    case VotePageDisplay.Wait:
      console.log("Got to Wait!");
      return WaitPage();
    case VotePageDisplay.Register:
      return <RegisterPage muid={muid} uuid={uuid} setVotePageDisplay={setVotePageDisplay} />;
    case VotePageDisplay.Dichotomous:
      return <DichotomousPage setVotePageDisplay={setVotePageDisplay} />
    default:
      setVotePageDisplay(VotePageDisplay.Wait);
  }
}

function WaitPage(): ReactElement {
  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="Waiting For Vote To Begin" description=<div>
        <p className="text-lg mb-6 opacity-80">
          You are a voter in this meeting. Please stand by and wait for the host to start a vote.
        </p>
      </div> />
      <Footer />
    </div>
  );
}

export default VoterPage;
