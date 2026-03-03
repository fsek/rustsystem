export const APIErrorCodes = {
  InvalidUUID: "InvalidUUID",
  InvalidMUID: "InvalidMUID",

  UUIDNotFound: "UUIDNotFound",
  MUIDNotFound: "MUIDNotFound",

  VoterNameNotFound: "VoterNameNotFound",

  UUIDAlreadyClaimed: "UUIDAlreadyClaimed",
  NameTaken: "NameTaken",
  AlreadyRegistered: "AlreadyRegistered",

  MUIDMismatch: "MUIDMismatch",

  InvalidMetaData: "InvalidMetaData",
  InvalidVoteMethod: "InvalidVoteMethod",
  InvalidVoteLength: "InvalidVoteLength",
  VotingInactive: "VotingInactive",

  SignatureInvalid: "SignatureInvalid",
  SignatureExpired: "SignatureExpired",
  SignatureFailure: "SignatureFailure",

  InvalidState: "InvalidState",
  AuthError: "AuthError",

  InvalidStatusCode: "InvalidStatusCode",
};

export type APIErrorCode = (typeof APIErrorCodes)[keyof typeof APIErrorCodes];

export type APIError = {
  error: {
    code: APIErrorCode;
    message: string;
    httpStatus: number;
    timestamp: string;
  };
  endpoint: {
    method: string;
    path: string;
  };
};

export class APIRequestError extends Error {
  readonly apiError: APIError;

  constructor(apiError: APIError) {
    super(apiError.error.message);
    this.name = "APIRequestError";
    this.apiError = apiError;
  }
}

/**
 * Parses the error body from a non-ok Response and throws an `APIRequestError`.
 * Falls back to a generic `Error("HTTP <status>")` if the body is not a valid
 * `APIError`.
 *
 * Must be called before consuming the response body (i.e. before `res.json()`).
 */
export async function handleErrorResponse(res: Response): Promise<never> {
  try {
    const apiError: APIError = await res.json();
    throw new APIRequestError(apiError);
  } catch (e) {
    if (e instanceof APIRequestError) throw e;
    throw new Error(`HTTP ${res.status}`);
  }
}
