import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useNavigate } from "@tanstack/react-router";
import { Login, LoginStatus } from "@/api/login";
import { matchResult } from "@/result";
import type { APIError } from "@/api/error";
import ErrorHandler from "@/components/error";

type SearchParams = {
  muuid: string;
  uuuid: string;
  admin_msg?: string;
  admin_sig?: string;
};

export const Route = createFileRoute("/login")({
  validateSearch: (search): SearchParams => {
    return {
      muuid: (search.muuid as string) ?? "",
      uuuid: (search.uuuid as string) ?? "",
      admin_msg: search.admin_msg as string,
      admin_sig: search.admin_sig as string,
    };
  },

  component: RouteComponent,
});

function RouteComponent() {
  const [loginStatus, setLoginStatus] = useState<LoginStatus>(
    LoginStatus.Loading,
  );
  const [error, setError] = useState<APIError | null>(null);
  const search = Route.useSearch();
  const navigate = useNavigate();

  const muuid = search.muuid;
  const uuuid = search.uuuid;
  const admin_msg = search.admin_msg;
  const admin_sig = search.admin_sig;

  useEffect(() => {
    // Parse admin credentials if provided (now hex-encoded)
    let admin_cred = undefined;
    if (admin_msg && admin_sig) {
      try {
        console.log("Raw admin_msg from URL:", admin_msg);
        console.log("Raw admin_sig from URL:", admin_sig);

        // Decode URL-encoded admin_msg (now hex-encoded)
        const decodedMsg = decodeURIComponent(admin_msg);
        console.log("Decoded admin_msg (hex):", decodedMsg);

        // Convert hex string to byte array
        const msgArray = [];
        for (let i = 0; i < decodedMsg.length; i += 2) {
          const byte = parseInt(decodedMsg.substr(i, 2), 16);
          if (isNaN(byte)) {
            throw new Error(
              `Invalid hex byte at position ${i}: ${decodedMsg.substr(i, 2)}`,
            );
          }
          msgArray.push(byte);
        }

        console.log("Parsed message array:", msgArray);

        if (msgArray.length !== 32) {
          throw new Error(`Expected 32 bytes, got ${msgArray.length}`);
        }

        admin_cred = {
          msg: msgArray,
          sig: decodeURIComponent(admin_sig),
        };
        console.log("Final admin credentials:", admin_cred);
      } catch (e) {
        console.error("Failed to parse admin credentials:", e);
      }
    }

    // Test admin credential parsing with sample data
    if (window.location.search.includes("test_admin=true")) {
      console.log("=== TESTING ADMIN CREDENTIAL PARSING (HEX) ===");
      const testMsg =
        "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"; // 32 bytes as hex
      const testSig = "test_signature_hex_string";

      try {
        const decodedMsg = decodeURIComponent(testMsg);
        console.log("Test decoded admin_msg (hex):", decodedMsg);

        const msgArray = [];
        for (let i = 0; i < decodedMsg.length; i += 2) {
          const byte = parseInt(decodedMsg.substr(i, 2), 16);
          msgArray.push(byte);
        }

        console.log("Test parsed message array:", msgArray);
        console.log("Test array length:", msgArray.length);

        const testCred = {
          msg: msgArray,
          sig: testSig,
        };
        console.log("Test admin credentials:", testCred);
      } catch (e) {
        console.error("Test parsing failed:", e);
      }
      console.log("=== END TEST ===");
    }

    Login({ muuid, uuuid, admin_cred }).then((result) => {
      matchResult(result, {
        Ok: (_res) => {
          setLoginStatus(LoginStatus.Success);
        },
        Err: (err) => {
          setError(err);
        },
      });
    });
  }, []);

  if (error) {
    return <ErrorHandler error={error} />;
  }
  if (loginStatus === LoginStatus.Loading) return <div>Kontrollerar...</div>;
  if (loginStatus === LoginStatus.Success) {
    navigate({ to: "/meeting", search: { muuid, uuuid } });
    return <div>Inloggad! Omdirigerar!</div>;
  }
}
