import { useLocation, useNavigate } from '@tanstack/react-router';
import React, { useEffect, useState } from "react";
import Button from '@/components/templates/button';
import FormSection from '@/components/templates/form';
import MainSection from '@/components/templates/main';
import { StartVote, type StartVoteRequest } from '@/api/host/state';
import init, { BallotMetaData, VoteMethod } from '@/pkg/rustsystem_client';
import { MeetingSpecs, type MeetingSpecsRequest, type MeetingSpecsResponse } from '@/api/common/meetingSpecs';
import { matchResult } from '@/result';

interface HostPageProps {
  muid: any,
}

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  init();
  const [specs, setSpecs] = useState<MeetingSpecsResponse | undefined>(undefined);

  const navigate = useNavigate();

  function invitePage() {
    navigate({ to: "/invite", search: { muid: muid } });
  }

  function startVote(data: Record<string, string>) {
    StartVote({ name: data.name, metadata: new BallotMetaData(VoteMethod.Dichotomous, 1) } as StartVoteRequest).then((result) => {
      matchResult(result, {
        Ok: (_res) => { },
        Err: (err) => { console.error(err) } // TODO: handle this error
      })
    });
  }

  // TODO: Change this into a SSE, so that the participants number is updated as more people join.
  useEffect(() => {
    MeetingSpecs({} as MeetingSpecsRequest).then((result) => {
      matchResult(result, {
        Ok: (s) => { setSpecs(s) },
        Err: (err) => { console.error(err) } // TODO: Handle this error
      })
    });
  }, []);

  return (
    <div>
      <Button label="Invite" fn={invitePage} />

      <MainSection title={specs ? specs.title : "Undefined"} description=<p>You are the host of this meeting</p> />
      <FormSection
        key={useLocation().pathname}
        submit={{ label: "Start Vote!", data: startVote }}
        fields={[
          { label: "name", id: "name", type: "text" },
        ]}
      />
    </div>
  );
}

export default HostPage;
