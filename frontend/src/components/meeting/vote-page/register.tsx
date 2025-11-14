import Footer from "@/components/defaults/footer";
import Header from "@/components/defaults/header";
import MainSection from "@/components/templates/main";

import {
  try_register,
  new_ballot_validation,
} from "@/pkg/rustsystem_client.js";
import { withWasm } from "@/utils/wasm";
import type React from "react";
import { useState } from "react";
import { VotePageDisplay } from "../voter";

type RegisterPageProps = {
  muid: any;
  uuuid: any;
  setVotePageDisplay: React.Dispatch<React.SetStateAction<VotePageDisplay>>;
};

const RegisterPage: React.FC<RegisterPageProps> = ({
  muid,
  uuuid,
  setVotePageDisplay,
}) => {
  const [isRegistering, setIsRegistering] = useState(false);

  async function sendRegistration() {
    setIsRegistering(true);
    try {
      await withWasm(async () => {
        const res = await try_register(muid, uuuid);
        if (res.is_valid() && res.is_successful()) {
          const validation = new_ballot_validation(
            res.proof(),
            res.token(),
            res.signature(),
          );
          console.log(validation.toValue());
          sessionStorage.setItem(
            "validation",
            JSON.stringify(validation.toValue()),
          );
          sessionStorage.setItem(
            "metadata",
            JSON.stringify(res.metadata()!.toValue()),
          );
          setVotePageDisplay(VotePageDisplay.Wait);
        } else {
          console.error("Registrering misslyckades");
        }
      });
    } catch (error) {
      console.error("WASM error during registration:", error);
    } finally {
      setIsRegistering(false);
    }
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-contours)] font-sans leading-relaxed transition-colors duration-500">
      <Header />

      <MainSection
        title="Registrera"
        description=<div>
          <p className="text-lg mb-6 opacity-80">
            En omröstning har startat! Du kan nu registrera dig.
          </p>
          <button
            type="button"
            onClick={sendRegistration}
            disabled={isRegistering}
            className="bg-[var(--color-main)] hover:bg-[var(--color-accent2)] disabled:bg-gray-400 disabled:cursor-not-allowed text-white py-3 px-6 rounded shadow-sm hover:shadow-md active:shadow-none active:translate-y-px transition-all duration-100"
          >
            {isRegistering ? "Registrerar..." : "Registrera"}
          </button>
        </div>
      />

      <Footer />
    </div>
  );
};

export default RegisterPage;
