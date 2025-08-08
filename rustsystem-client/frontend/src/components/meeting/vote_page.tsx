import init, { register, send_vote, RegistrationResult } from "@/pkg/rustsystem_client.js";
import Button from "@/components/templates/button";

type VotePageProps = {
  muid: any,
  uuid: any,
}

const VotePage: React.FC<VotePageProps> = ({ muid, uuid }) => {
  init();

  async function sendRegistration() {
    const res = await register(muid, uuid);
    sessionStorage.setItem("proof", JSON.stringify(res.proof()));
    sessionStorage.setItem("token", JSON.stringify(res.token()));
    sessionStorage.setItem("signature", JSON.stringify(res.signature()));
  }

  async function validate() {
    const proof = new Uint8Array(Object.values(JSON.parse(sessionStorage.getItem("proof")!)));
    const token = new Uint8Array(Object.values(JSON.parse(sessionStorage.getItem("token")!)));
    const signature = JSON.parse(sessionStorage.getItem("signature")!);
    console.log(proof);
    console.log(token);
    console.log(signature);
    const res = await send_vote(RegistrationResult.with_signature(proof, token, signature));
    console.log(res);
  }

  return (
    <div>
      <p>You can now Register</p>
      <Button label="Register" fn={sendRegistration} />
      <Button label="Validate" fn={validate} />
    </div>
  )
}

export default VotePage;
