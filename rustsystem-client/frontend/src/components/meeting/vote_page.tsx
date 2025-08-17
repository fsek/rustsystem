import init, { try_register, send_vote, WASMChoice, new_ballot_validation } from "@/pkg/rustsystem_client.js";
import Button from "@/components/templates/button";

type VotePageProps = {
  muid: any,
  uuid: any,
}

const VotePage: React.FC<VotePageProps> = ({ muid, uuid }) => {
  init();

  async function sendRegistration() {
    const res = await try_register(muid, uuid);
    if (res.is_valid() && res.is_successful()) {
      const validation = new_ballot_validation(res.proof(), res.token(), res.signature());
      sessionStorage.setItem("validation", JSON.stringify(validation.toValue()));
      sessionStorage.setItem("metadata", JSON.stringify(res.metadata()!.toValue()))
    } else {
      // TODO: This should be handled such that the user knows that the registration failed
      console.error("Registration was unsuccessful");
    }
  }

  async function validate() {
    const proof = new Uint8Array(Object.values(JSON.parse(sessionStorage.getItem("proof")!)));
    const token = new Uint8Array(Object.values(JSON.parse(sessionStorage.getItem("token")!)));
    const signature = JSON.parse(sessionStorage.getItem("signature")!);
    console.log(proof);
    console.log(token);
    console.log(signature);

    const validation = JSON.parse(sessionStorage.getItem("validation")!);
    const metadata = JSON.parse(sessionStorage.getItem("metadata")!);
    const choice = new WASMChoice();

    console.log(metadata);
    console.log(choice);
    console.log(validation);

    const res = await send_vote(metadata, choice, validation);
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
