import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import type { APIError } from "@/api/error";
import { StartVote, type StartVoteRequest } from "@/api/host/state";
import FormSection from "@/components/templates/form";
import MainSection from "@/components/templates/main";
import init, { BallotMetaData, VoteMethod } from "@/pkg/rustsystem_client";
import { matchResult } from "@/result";
import { useLocation } from "@tanstack/react-router";
import type React from "react";
import type { HostPageDisplay } from "../host";

type CreationPageProps = {
  specs: MeetingSpecsResponse | undefined;
  muid: any;
  setHostPageDisplay: React.Dispatch<React.SetStateAction<HostPageDisplay>>;
  setError: React.Dispatch<React.SetStateAction<APIError | null>>;
};

const CreationPage: React.FC<CreationPageProps> = ({ specs, setError }) => {
  init();
  console.log("Got to creation");
  const location = useLocation();

  function startVote(data: Record<string, string>) {
    StartVote({
      name: data.name,
      metadata: new BallotMetaData(VoteMethod.Dichotomous, 1),
    } as StartVoteRequest).then((result) => {
      matchResult(result, {
        Ok: (_res) => {},
        Err: (err) => {
          setError(err);
        },
      });
    });
  }

  return (
    <div>
      <MainSection
        title={specs ? specs.title : "Undefined"}
        description=<p>
          You are the host of this meeting. There are{" "}
          {specs ? specs.participants : "unknown"} participants in this meeting
        </p>
      />
      <FormSection
        key={location.pathname}
        submit={{ label: "Start Vote!", data: startVote }}
        fields={[{ label: "name", id: "name", type: "text" }]}
      />
    </div>
  );
};

export default CreationPage;
