export type CreateMeetingRequest = {
  title: string;
};

type CreateMeetingResponse = {
  muid?: any;
  uuid?: any;
};

export async function CreateMeeting(
  req: CreateMeetingRequest,
): Promise<CreateMeetingResponse> {
  console.log("got title", req.title);
  const res = await fetch("api/create-meeting", {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const data = await res.json();
  return data as CreateMeetingResponse;
}
