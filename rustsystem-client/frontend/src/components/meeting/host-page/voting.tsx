import MainSection from "@/components/templates/main";
import type { MeetingSpecsResponse } from "@/api/common/meetingSpecs";
import Button from "@/components/templates/button";
import { Tally, type TallyRequest, type TallyResponse } from "@/api/host/state";
import { matchResult } from "@/result";
import type React from "react";
import type { APIError } from "@/api/error";
import { HostPageDisplay } from "../host";

type VotingPageProps = {
  specs: MeetingSpecsResponse | undefined;
  setTally: React.Dispatch<React.SetStateAction<TallyResponse | null>>;
  setHostPageDisplay: React.Dispatch<React.SetStateAction<HostPageDisplay>>,
  setError: React.Dispatch<React.SetStateAction<APIError | null>>,
}

const VotingPage: React.FC<VotingPageProps> = ({ specs, setTally, setHostPageDisplay, setError }) => {
  console.log("Got to voting");
  function tally() {
    Tally({} as TallyRequest).then((result) => {
      matchResult(result, {
        Ok: (res) => {
          setTally(res);
          setHostPageDisplay(HostPageDisplay.Tally);
        },
        Err: (err) => { setError(err) },
      })
    });
  }

  return (
    <div>
      <MainSection title={specs ? specs.title : "Undefined"} description=<div>
        <p>The voting round is now active. Participants can vote.</p>
        <Button label="Tally" fn={tally} />
      </div> />
    </div>
  );
}

export default VotingPage;
