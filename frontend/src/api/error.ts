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
