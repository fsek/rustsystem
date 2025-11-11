import type { APIError } from "@/api/error";
import { startInviteWait } from "@/api/host/inviteEvent";
import {
  newVoter,
  startInvite,
  type startInviteRequest,
} from "@/api/host/newVoter";
import { matchResult } from "@/result";
import type React from "react";
import { useEffect, useState } from "react";
import ErrorHandler from "../error";

export const RunInvite: React.FC = () => {
  const [qrSvg, setQrSvg] = useState<string | undefined>(undefined);
  const [error, setError] = useState<APIError | null>(null);

  async function connectStart() {
    return new Promise((resolve, reject) => {
      const inviteEvent = startInviteWait();
      inviteEvent.onmessage = (event) => {
        if (event.data == "Ready") {
          get_qr_svg().then((svg) => {
            setQrSvg(svg);
          });
        }
      };

      inviteEvent.onerror = (err) => {
        reject(err);
      };

      inviteEvent.onopen = () => {
        resolve(inviteEvent);
        startInvite({} as startInviteRequest).then((result) => {
          matchResult(result, {
            Ok: (_res) => {},
            Err: (err) => {
              setError(err);
            },
          });
        });
      };
    });
  }

  async function get_qr_svg(): Promise<string> {
    const result = await newVoter({ voterName: "Bert", isHost: false });
    return matchResult(result, {
      Ok: (res) => {
        return res.qrSvg;
      },
      Err: (err) => {
        setError(err);
        return "Could not get QR code";
      },
    });
  }

  useEffect(() => {
    connectStart();
  }, []);

  if (error) {
    return <ErrorHandler error={error} />;
  }

  return (
    <div>
      Access Granted!
      {qrSvg ? (
        <div dangerouslySetInnerHTML={{ __html: qrSvg }} />
      ) : (
        <div>Could not load QR code</div>
      )}
    </div>
  );
};

export default RunInvite;
