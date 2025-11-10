import Footer from "@/components/defaults/footer";
import Header from "@/components/defaults/header";
import Button from "@/components/templates/button";
import MainSection from "@/components/templates/main";

import init, {
	try_register,
	new_ballot_validation,
} from "@/pkg/rustsystem_client.js";
import type React from "react";
import { VotePageDisplay } from "../voter";

type RegisterPageProps = {
	muid: any;
	uuid: any;
	setVotePageDisplay: React.Dispatch<React.SetStateAction<VotePageDisplay>>;
};

const RegisterPage: React.FC<RegisterPageProps> = ({
	muid,
	uuid,
	setVotePageDisplay,
}) => {
	init();

	async function sendRegistration() {
		const res = await try_register(muid, uuid);
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
			setVotePageDisplay(VotePageDisplay.Wait); // TODO: Switch to active voting page
		} else {
			// TODO: Detta bör hanteras så att användaren vet att registreringen misslyckades
			console.error("Registrering misslyckades");
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
					<Button label="Registrera" fn={sendRegistration} />
				</div>
			/>

			<Footer />
		</div>
	);
};

export default RegisterPage;
