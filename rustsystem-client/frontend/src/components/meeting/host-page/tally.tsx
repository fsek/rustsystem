import type React from "react"
import { HostPageDisplay } from "../host"
import type { APIError } from "@/api/error"
import MainSection from "@/components/templates/main"
import { EndVoteRound, type EndVoteRoundRequest, type TallyResponse } from "@/api/host/state";
import Button from "@/components/templates/button";

type TallyPageProps = {
  tally: TallyResponse | null,
  setHostPageDisplay: React.Dispatch<React.SetStateAction<HostPageDisplay>>,
  setError: React.Dispatch<React.SetStateAction<APIError | null>>,
};

const TallyPage: React.FC<TallyPageProps> = ({ tally, setHostPageDisplay }) => {
  console.log("Got to tally");
  let result;
  if (tally) {
    if ('Dichotomous' in tally.score) {
      const score = tally.score['Dichotomous'] as Array<number>;
      result = <DichotomousTally yes={score[0]} no={score[1]}></DichotomousTally>
    }
  }

  function backToMeeting() {
    EndVoteRound({} as EndVoteRoundRequest).then((_res) => {
      setHostPageDisplay(HostPageDisplay.Creation);
    });
  }

  return (
    <div>
      <MainSection title="Results" description=<div>{result}<p>Blank: {tally?.blank}</p></div>
      />
      <Button label="Back to Meeting" fn={backToMeeting} />
    </div>
  );
}


type DichotomousTallyProps = {
  yes: number,
  no: number,
}
const DichotomousTally: React.FC<DichotomousTallyProps> = ({ yes, no }) => {
  return (
    <div>
      <p>
        Yes: {yes}
      </p>
      <p>
        No: {no}
      </p>
    </div>
  )
}

export default TallyPage;
