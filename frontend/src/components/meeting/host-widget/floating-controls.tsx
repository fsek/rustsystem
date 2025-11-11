import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useEffect, useState } from "react";

import {
  MeetingSpecs,
  type MeetingSpecsRequest,
  type MeetingSpecsResponse,
  meetingSpecsWatch,
} from "@/api/common/meetingSpecs";

import { VoterList, type VoterListRequest } from "@/api/host/voterList";
import type { APIError } from "@/api/error";
import { matchResult } from "@/result";
import "@/colors.css";

interface FloatingControlsProps {
  muid: string;
  setError: (error: APIError) => void;
}

const FloatingControls: React.FC<FloatingControlsProps> = ({
  muid,
  setError,
}) => {
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(
    undefined,
  );
  const [participantCount, setParticipantCount] = useState(0);
  const navigate = useNavigate();

  function fetchSpecs() {
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (s) => {
          setSpecs(s);
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }

  function fetchVoterCount() {
    VoterList({} as VoterListRequest).then((result) => {
      matchResult(result, {
        Ok: (response) => {
          setParticipantCount(response.voters.length);
        },
        Err: (err) => {
          console.error("Failed to fetch voter list:", err);
          // Fallback to specs participants if available
          setParticipantCount(specs?.participants || 0);
        },
      });
    });
  }

  useEffect(() => {
    fetchSpecs();
    fetchVoterCount();

    const specsEvent = meetingSpecsWatch();
    specsEvent.onmessage = (event) => {
      if (event.data === "NewData") {
        fetchSpecs();
      }
    };

    // Refresh voter count every 10 seconds to keep it updated
    const voterInterval = setInterval(fetchVoterCount, 10000);

    return () => {
      specsEvent.close();
      clearInterval(voterInterval);
    };
  }, []);

  const handleInvite = () => {
    navigate({ to: "/invite", search: { muid } });
  };

  return (
    <div className="fixed bottom-6 right-6 bg-white border border-gray-200 rounded-lg shadow-lg p-4 flex items-center gap-4 z-50">
      {/* Voter Count */}
      <div className="flex items-center gap-2">
        <div className="w-3 h-3 bg-green-500 rounded-full"></div>
        <span className="text-sm font-medium text-gray-700">
          {participantCount} deltagare
        </span>
      </div>

      {/* Divider */}
      <div className="w-px h-6 bg-gray-200"></div>

      {/* Invite Button */}
      <button
        onClick={handleInvite}
        className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] text-white px-3 py-2 rounded text-sm font-medium shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
      >
        Bjud in
      </button>
    </div>
  );
};

export default FloatingControls;
