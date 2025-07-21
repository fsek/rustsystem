import { useNavigate } from '@tanstack/react-router';
import React from "react";
import Button from '@/components/templates/button';

interface HostPageProps {
  muid: any,
}

const HostPage: React.FC<HostPageProps> = ({ muid }) => {
  const navigate = useNavigate();

  function invitePage() {
    navigate({ to: "/invite", search: { muid: muid } });
  }

  return (
    <div>
      <Button label="Invite" fn={invitePage} />
    </div>
  );
}

export default HostPage;
