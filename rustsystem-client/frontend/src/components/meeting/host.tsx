import { useLocation, useNavigate } from '@tanstack/react-router';
import React from "react";
import Button from '@/components/templates/button';
import FormSection from '@/components/templates/form';
import { StartVote, type StartVoteRequest } from '@/api/host/state';
import init, { BallotMetaData, VoteMethod } from '@/pkg/rustsystem_client';

interface HostPageProps {
  muid: any,
}

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  init();

  const navigate = useNavigate();

  function invitePage() {
    navigate({ to: "/invite", search: { muid: muid } });
  }

  function startVote(data: Record<string, string>) {
    StartVote({ name: data.name, metadata: new BallotMetaData(VoteMethod.Dichotomous, 1) } as StartVoteRequest);
  }

  return (
    <div>
      <Button label="Invite" fn={invitePage} />
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
