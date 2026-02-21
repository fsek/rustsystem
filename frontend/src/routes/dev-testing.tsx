import { createFileRoute } from "@tanstack/react-router";
import {
  deriveEd25519PublicKeyFromPassword,
  x25519PublicKeyToPem,
} from "@/utils/cryptoGen";
import { Button } from "@/components/Button/Button";

export const Route = createFileRoute("/dev-testing")({
  component: DevTesting,
});

function DevTesting() {
  function doDerive() {
    const password = "Epic Password";
    const saltHex = "46bce114c7f20a9e14bbf2a41aa6236d";
    const iterations = 200000;

    deriveEd25519PublicKeyFromPassword({
      password,
      saltHex,
      iterations,
    }).then((publicKey) => {
      console.log("pub: ", x25519PublicKeyToPem(publicKey));
    });
  }

  return (
    <Button size="l" color="buttonPrimary" onClick={doDerive}>
      Derive
    </Button>
  );
}
