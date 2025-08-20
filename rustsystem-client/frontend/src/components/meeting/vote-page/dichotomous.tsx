import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from '@/components/templates/main';
import Button from "@/components/templates/button";

import init, { WASMChoice, send_vote } from "@/pkg/rustsystem_client.js";
import type { VotePageDisplay } from '../voter';


type DichotomousProps = {
  setVotePageDisplay: React.Dispatch<React.SetStateAction<VotePageDisplay>>,
}

const DichotomousPage: React.FC<DichotomousProps> = ({ setVotePageDisplay }) => {
  console.log("Got to Vote Page!");
  console.log(setVotePageDisplay);
  init();

  async function voteYes() {
    await validate(true);
  }

  async function voteNo() {
    await validate(false);
  }

  async function voteBlank() {
    await validate(null);
  }

  async function validate(vote: boolean | null) {
    console.log("Got here 1");
    const choice = new WASMChoice();
    console.log("Got here 2");
    choice.set_dichotomous(vote);
    console.log("Got here 3");

    const validation = JSON.parse(sessionStorage.getItem("validation")!);
    const metadata = JSON.parse(sessionStorage.getItem("metadata")!);

    console.log(metadata);
    console.log(choice);
    console.log(validation);

    await send_vote(metadata, choice, validation);
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="Vote" description=<div>
        <p className="text-lg mb-6 opacity-80">You have successfully registered and can now cast your vote</p>
        <Button label="Yes" fn={voteYes} />
        <Button label="No" fn={voteNo} />
        <Button label="Blank" fn={voteBlank} />
      </div> />
      {/* <Button label="Validate" fn={validate} /> */}

      <Footer />
    </div>
  )
}

export default DichotomousPage;
