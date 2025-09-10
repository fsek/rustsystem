import React, { useEffect, useState } from "react";
import { MeetingSpecs, type MeetingSpecsRequest, type MeetingSpecsResponse } from '@/api/common/meetingSpecs';
import { matchResult } from '@/result';
import type { APIError } from '@/api/error';
import ErrorHandler from '../error';
import CreationPage from './host-page/creation';
import VotingPage from "./host-page/voting";
import { voteStateWatch } from "@/api/common/state";

interface HostPageProps {
  muid: any,
}

export const HostPageDisplay = {
  // Stages of the voting process (from the host side)
  Creation: 1,
  Voting: 2,
  Tally: 3,
} as const;
export type HostPageDisplay = (typeof HostPageDisplay)[keyof typeof HostPageDisplay];

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  const voteStateEvent = voteStateWatch();
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(undefined);
  const [currentHostPageDisplay, setHostPageDisplay] = useState<HostPageDisplay>(HostPageDisplay.Creation)
  const [error, setError] = useState<APIError | null>(null);

  voteStateEvent.onmessage = function (event) {
    if (currentHostPageDisplay === HostPageDisplay.Creation) {
      if (event.data === "Creation") {
        setHostPageDisplay(HostPageDisplay.Creation)
      } else if (event.data === "Voting") {
        setHostPageDisplay(HostPageDisplay.Voting)
      } else if (event.data === "Tally") {
        setHostPageDisplay(HostPageDisplay.Tally)
      }
    }
  }

  // TODO: Change this into a SSE, so that the participants number is updated as more people join.
  useEffect(() => {
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (s) => { setSpecs(s) },
        Err: (err) => { setError(err) }
      })
    });
  }, []);

  if (error) {
    return <ErrorHandler error={error} />
  }
  switch (currentHostPageDisplay) {
    case HostPageDisplay.Creation:
      return <CreationPage specs={specs} muid={muid} setHostPageDisplay={setHostPageDisplay} setError={setError} />;
    case HostPageDisplay.Voting:
      return <VotingPage specs={specs} setHostPageDisplay={setHostPageDisplay} setError={setError} />
    default:
      setHostPageDisplay(HostPageDisplay.Creation);
  }
}

export default HostPage;
