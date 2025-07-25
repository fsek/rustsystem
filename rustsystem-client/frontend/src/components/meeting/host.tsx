import { useLocation, useNavigate } from '@tanstack/react-router';
import React from "react";
import Button from '@/components/templates/button';
import FormSection from '@/components/templates/form';

interface HostPageProps {
  muid: any,
}

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  const navigate = useNavigate();

  function invitePage() {
    navigate({ to: "/invite", search: { muid: muid } });
  }
  function startVote(data: Record<string, string>) {
    fetch("api/start-vote", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data)
    });
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
