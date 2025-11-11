import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import { Tally, type TallyRequest, type TallyResponse } from "@/api/host/state";
import { VoterList, type VoterListRequest } from "@/api/host/voterList";
import { matchResult } from "@/result";
import type React from "react";
import { useEffect, useState } from "react";
import { VoteState } from "../host";

type VotingPageProps = {
  specs: MeetingSpecsResponse | undefined;
  setTally: React.Dispatch<React.SetStateAction<TallyResponse | null>>;
  setCurrentState: React.Dispatch<React.SetStateAction<VoteState>>;
  setError: React.Dispatch<React.SetStateAction<APIError | null>>;
  muid: string;
};

interface Voter {
  name: string;
  uuid: string;
  loggedIn: boolean;
}

const VotingPage: React.FC<VotingPageProps> = ({
  specs,
  setTally,
  setCurrentState,
  setError,
}) => {
  const [isLoading, setIsLoading] = useState(false);
  const [voters, setVoters] = useState<Voter[]>([]);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  // Fetch voters periodically to show real-time status
  useEffect(() => {
    const fetchVoters = async () => {
      const result = await VoterList({} as VoterListRequest);
      matchResult(result, {
        Ok: (response) => {
          const votersData = response.voters.map((voter) => ({
            name: voter.name,
            uuid: voter.uuid,
            loggedIn: voter.loggedIn,
          }));
          setVoters(votersData);
          setLastRefresh(new Date());
        },
        Err: (err) => {
          // Don't show error for voter list fetching during voting
          console.error("Failed to fetch voters:", err);
        },
      });
    };

    fetchVoters();
    const interval = setInterval(fetchVoters, 5000); // Refresh every 5 seconds

    return () => clearInterval(interval);
  }, []);

  const handleTally = async () => {
    setIsLoading(true);

    const result = await Tally({} as TallyRequest);

    matchResult(result, {
      Ok: (res) => {
        setTally(res);
        setCurrentState(VoteState.Tally);
        setIsLoading(false);
      },
      Err: (err) => {
        setError(err);
        setIsLoading(false);
      },
    });
  };

  const checkedInVoters = voters.filter((v) => v.loggedIn);
  const totalVoters = voters.length;

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            {specs?.title || "Meeting"}
          </h1>
          <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
            <span className="flex items-center gap-1">
              👥 {totalVoters} deltagare
            </span>
            <span className="flex items-center gap-1">
              ✅ {checkedInVoters.length} checked in
            </span>
          </div>
        </div>

        {/* Status Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Omröstning aktiv
              </h2>
              <p className="text-gray-600">
                Deltagare kan nu skicka sina röster
              </p>
            </div>
          </div>

          {/* Progress Bar */}
          <div className="mb-4">
            <div className="flex justify-between text-sm text-gray-600 mb-2">
              <span>Deltagande</span>
              <span>
                {checkedInVoters.length}/{totalVoters}
              </span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className="bg-green-500 h-2 rounded-full transition-all duration-500"
                style={{
                  width:
                    totalVoters > 0
                      ? `${(checkedInVoters.length / totalVoters) * 100}%`
                      : "0%",
                }}
              ></div>
            </div>
          </div>
        </div>

        {/* Voter Status Grid */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-xl font-semibold text-gray-900">
              Live deltagarstatus
            </h3>
            <div className="text-xs text-gray-500">
              Senast uppdaterad: {lastRefresh.toLocaleTimeString()}
            </div>
          </div>

          {voters.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {voters.map((voter) => (
                <div
                  key={voter.uuid}
                  className={`flex items-center gap-3 p-3 rounded-lg border ${
                    voter.loggedIn
                      ? "bg-green-50 border-green-200"
                      : "bg-gray-50 border-gray-200"
                  }`}
                >
                  <div
                    className={`w-2 h-2 rounded-full ${
                      voter.loggedIn ? "bg-green-500" : "bg-gray-400"
                    }`}
                  ></div>
                  <span className="text-sm font-medium text-gray-900">
                    {voter.name}
                  </span>
                  <span
                    className={`ml-auto text-xs px-2 py-1 rounded-full ${
                      voter.loggedIn
                        ? "bg-green-100 text-green-800"
                        : "bg-gray-100 text-gray-600"
                    }`}
                  >
                    {voter.loggedIn ? "Online" : "Offline"}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500">
              <p>Inga deltagare hittades</p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-lg font-semibold text-gray-900">
                Redo att avsluta omröstningen?
              </h3>
              <p className="text-gray-600 text-sm">
                Detta kommer att stänga omröstningen och beräkna resultat
              </p>
            </div>
            <button
              onClick={handleTally}
              disabled={isLoading}
              className="px-6 py-3 bg-red-600 hover:bg-red-700 disabled:bg-red-300 text-white font-medium rounded-md shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100 disabled:cursor-not-allowed"
            >
              {isLoading ? (
                <span className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Beräknar resultat...
                </span>
              ) : (
                "Avsluta omröstning & räkna"
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default VotingPage;
