import React, { useEffect, useState, type ReactElement } from "react";
import RegisterPage from "@/components/meeting/vote-page/register";
import VotingPage from "@/components/meeting/vote-page/voting";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  VoteActive,
  voteStateWatch,
  type VoteActiveRequest,
} from "@/api/common/state";
import {
  MeetingSpecs,
  meetingSpecsWatch,
  type MeetingSpecsRequest,
  type MeetingSpecsResponse,
} from "@/api/common/meetingSpecs";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "../error";

type VoterPageProps = {
  muid: any;
  uuid: any;
};

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
  Voting: 7,
} as const;
export type VotePageDisplay =
  (typeof VotePageDisplay)[keyof typeof VotePageDisplay];

const VoterPage: React.FC<VoterPageProps> = ({ muid, uuid }) => {
  const voteStateEvent = voteStateWatch();
  const [currentVotePageDisplay, setVotePageDisplay] =
    useState<VotePageDisplay>(VotePageDisplay.Wait);
  const [error, setError] = useState<APIError | null>(null);
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(
    undefined,
  );

  useEffect(() => {
    // Fetch meeting specs to get title and agenda
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (specsData) => {
          setSpecs(specsData);
        },
        Err: (err) => {
          setError(err);
        },
      });
    });

    // Explicitly check for voteActive being true.
    VoteActive({} as VoteActiveRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          if (res.isActive === true) {
            setVotePageDisplay(VotePageDisplay.Voting);
          } else {
            setVotePageDisplay(VotePageDisplay.Wait);
          }
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }, []);

  // Watch for meeting specs updates
  useEffect(() => {
    const specsEvent = meetingSpecsWatch();

    specsEvent.onmessage = function (event) {
      if (event.data === "NewData") {
        MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
          matchResult(result, {
            Ok: (specsData) => setSpecs(specsData),
            Err: (err) => setError(err),
          });
        });
      }
    };

    return () => {
      specsEvent.close();
    };
  }, []);

  voteStateEvent.onmessage = function (event) {
    if (currentVotePageDisplay === VotePageDisplay.Wait)
      if (event.data === "Voting") {
        setVotePageDisplay(VotePageDisplay.Voting);
      } else if (event.data === "Creation" || event.data === "Tally") {
        setVotePageDisplay(VotePageDisplay.Wait);
      }
  };

  if (error) {
    return <ErrorHandler error={error} />;
  }
  switch (currentVotePageDisplay) {
    case VotePageDisplay.Wait:
      return WaitPage(specs);
    case VotePageDisplay.Register:
      return (
        <RegisterPage
          muid={muid}
          uuid={uuid}
          setVotePageDisplay={setVotePageDisplay}
        />
      );
    case VotePageDisplay.Voting:
      return (
        <VotingPage
          muid={muid}
          uuid={uuid}
          specs={specs}
          setVotePageDisplay={setVotePageDisplay}
          setError={setError}
        />
      );
    default:
      setVotePageDisplay(VotePageDisplay.Wait);
  }
};

function WaitPage(specs: MeetingSpecsResponse | undefined): ReactElement {
  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            {specs?.title || "Möte"}
          </h1>
          <div className="flex items-center justify-center gap-4 text-sm text-gray-600">
            <span className="flex items-center gap-1">
              👥 {specs?.participants || 0} deltagare
            </span>
            <span className="flex items-center gap-1">⏳ Väntar på start</span>
          </div>
        </div>

        {/* Status Card */}
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-8">
          <div className="flex items-center gap-3">
            <div className="w-3 h-3 bg-yellow-500 rounded-full animate-pulse"></div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Väntar på omröstning
              </h2>
              <p className="text-gray-600">
                Värden förbereder omröstningen. Vänligen vänta.
              </p>
            </div>
          </div>
        </div>

        {/* Agenda Display */}
        {specs?.agenda && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
            <h3 className="text-xl font-semibold text-gray-900 mb-4">
              Mötesagenda
            </h3>
            <div className="prose prose-gray max-w-none">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  h1: ({ children }) => (
                    <h1 className="text-2xl font-bold text-gray-900 mt-6 mb-4 first:mt-0">
                      {children}
                    </h1>
                  ),
                  h2: ({ children }) => (
                    <h2 className="text-xl font-semibold text-gray-800 mt-5 mb-3">
                      {children}
                    </h2>
                  ),
                  h3: ({ children }) => (
                    <h3 className="text-lg font-medium text-gray-800 mt-4 mb-2">
                      {children}
                    </h3>
                  ),
                  p: ({ children }) => (
                    <p className="text-gray-700 mb-3 leading-relaxed">
                      {children}
                    </p>
                  ),
                  ul: ({ children }) => (
                    <ul className="list-disc list-inside text-gray-700 mb-3 space-y-1">
                      {children}
                    </ul>
                  ),
                  ol: ({ children }) => (
                    <ol className="list-decimal list-inside text-gray-700 mb-3 space-y-1">
                      {children}
                    </ol>
                  ),
                  li: ({ children }) => (
                    <li className="text-gray-700">{children}</li>
                  ),
                  blockquote: ({ children }) => (
                    <blockquote className="border-l-4 border-blue-200 pl-4 py-2 mb-3 bg-blue-50 text-gray-700 italic">
                      {children}
                    </blockquote>
                  ),
                  code: ({ children, className }) => {
                    const isBlock = className?.includes("language-");
                    if (isBlock) {
                      return (
                        <pre className="bg-gray-100 rounded p-3 mb-3 overflow-x-auto">
                          <code className="text-sm text-gray-800 font-mono">
                            {children}
                          </code>
                        </pre>
                      );
                    }
                    return (
                      <code className="bg-gray-100 px-1 py-0.5 rounded text-sm font-mono text-gray-800">
                        {children}
                      </code>
                    );
                  },
                  strong: ({ children }) => (
                    <strong className="font-semibold text-gray-900">
                      {children}
                    </strong>
                  ),
                  em: ({ children }) => (
                    <em className="italic text-gray-700">{children}</em>
                  ),
                  hr: () => <hr className="border-gray-300 my-6" />,
                  table: ({ children }) => (
                    <div className="overflow-x-auto mb-4">
                      <table className="min-w-full border border-gray-300">
                        {children}
                      </table>
                    </div>
                  ),
                  thead: ({ children }) => (
                    <thead className="bg-gray-50">{children}</thead>
                  ),
                  tbody: ({ children }) => (
                    <tbody className="bg-white">{children}</tbody>
                  ),
                  tr: ({ children }) => (
                    <tr className="border-b border-gray-200">{children}</tr>
                  ),
                  th: ({ children }) => (
                    <th className="px-4 py-2 text-left font-semibold text-gray-900 border-r border-gray-300 last:border-r-0">
                      {children}
                    </th>
                  ),
                  td: ({ children }) => (
                    <td className="px-4 py-2 text-gray-700 border-r border-gray-300 last:border-r-0">
                      {children}
                    </td>
                  ),
                }}
              >
                {specs.agenda}
              </ReactMarkdown>
            </div>
          </div>
        )}

        {!specs?.agenda && (
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
            <div className="text-gray-400 mb-2">📋</div>
            <p className="text-gray-500">
              No agenda has been set for this meeting
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

export default VoterPage;
