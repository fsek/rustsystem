import React, { useState } from "react";

import init, { register, RegistrationResult } from "@/pkg/rustsystem_client.js";
import Button from "@/components/templates/button";

type VotePageProps = {
  muid: any,
  uuid: any,
}

const VotePage: React.FC<VotePageProps> = ({ muid, uuid }) => {
  init();
  const [ballot, setBallot] = useState<RegistrationResult | undefined>(undefined);

  async function sendRegistration() {
    const res = await register(muid, uuid);
    setBallot(res);
  }

  return (
    <div>
      <p>You can now Register</p>
      <Button label="Register" fn={sendRegistration} />
      {JSON.stringify(ballot?.signature())}
      {JSON.stringify(ballot?.proof())}
    </div>
  )
}

export default VotePage;
