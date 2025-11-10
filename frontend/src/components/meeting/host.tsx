import React, { useEffect, useState } from "react";
import {
  MeetingSpecs,
  meetingSpecsWatch,
  type MeetingSpecsRequest,
  type MeetingSpecsResponse,
} from "@/api/common/meetingSpecs";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "../error";
import CreationPage from "./host-page/creation";
import VotingPage from "./host-page/voting";
import { voteStateWatch } from "@/api/common/state";
import TallyPage from "./host-page/tally";
import type { TallyResponse } from "@/api/host/state";

interface HostPageProps {
  muid: string;
}

export enum VoteState {
  Creation = "Creation",
  Voting = "Voting",
  Tally = "Tally",
}

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(
    undefined,
  );
  const [currentState, setCurrentState] = useState<VoteState>(
    VoteState.Creation,
  );
  const [error, setError] = useState<APIError | null>(null);
  const [tally, setTally] = useState<TallyResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Fetch meeting specs
  useEffect(() => {
    const fetchSpecs = async () => {
      const result = await MeetingSpecs({} as MeetingSpecsRequest);
      matchResult(result, {
        Ok: (s) => {
          setSpecs(s);
          setIsLoading(false);
        },
        Err: (err) => {
          setError(err);
          setIsLoading(false);
        },
      });
    };

    fetchSpecs();
  }, []);

  // Watch for vote state changes
  useEffect(() => {
    const voteStateEvent = voteStateWatch();

    voteStateEvent.onmessage = function (event) {
      const newState = event.data as string;
      if (Object.values(VoteState).includes(newState as VoteState)) {
        setCurrentState(newState as VoteState);
      }
    };

    return () => {
      voteStateEvent.close();
    };
  }, []);

  // Watch for meeting specs updates
  useEffect(() => {
    const specsEvent = meetingSpecsWatch();

    specsEvent.onmessage = function (event) {
      if (event.data === "NewData") {
        MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
          matchResult(result, {
            Ok: (s) => setSpecs(s),
            Err: (err) => setError(err),
          });
        });
      }
    };

    return () => {
      specsEvent.close();
    };
  }, []);

  if (error) {
    return <ErrorHandler error={error} />;
  }

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading meeting...</p>
        </div>
      </div>
    );
  }

  const commonProps = {
    specs,
    muid,
    setError,
    setTally,
    currentState,
    setCurrentState,
  };

  switch (currentState) {
    case VoteState.Creation:
      return <CreationPage {...commonProps} />;
    case VoteState.Voting:
      return <VotingPage {...commonProps} />;
    case VoteState.Tally:
      return <TallyPage {...commonProps} tally={tally} />;
    default:
      return <CreationPage {...commonProps} />;
  }
};

export default HostPage;
