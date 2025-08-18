import Header from '@/components/defaults/header';
import Footer from '@/components/defaults/footer';
import MainSection from '@/components/templates/main';
import Button from "@/components/templates/button";

import init, { try_register, new_ballot_validation } from "@/pkg/rustsystem_client.js";
import { VotePageDisplay } from '../voter';
import type React from 'react';


type RegisterPageProps = {
  muid: any,
  uuid: any,
  setVotePageDisplay: React.Dispatch<React.SetStateAction<VotePageDisplay>>,
}

const RegisterPage: React.FC<RegisterPageProps> = ({ muid, uuid, setVotePageDisplay }) => {
  init();

  async function sendRegistration() {
    const res = await try_register(muid, uuid);
    if (res.is_valid() && res.is_successful()) {
      const validation = new_ballot_validation(res.proof(), res.token(), res.signature());
      sessionStorage.setItem("validation", JSON.stringify(validation.toValue()));
      sessionStorage.setItem("metadata", JSON.stringify(res.metadata()!.toValue()))
      setVotePageDisplay(VotePageDisplay.Dichotomous);
    } else {
      // TODO: This should be handled such that the user knows that the registration failed
      console.error("Registration was unsuccessful");
    }
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection title="Register" description=<div>
        <p className="text-lg mb-6 opacity-80">A vote has started! You can now register.</p>
        <Button label="Register" fn={sendRegistration} />
      </div> />

      <Footer />
    </div>
  )
}

export default RegisterPage;
