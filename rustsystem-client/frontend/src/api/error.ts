export const APIErrorCodes = {
  InvalidUUID: "InvalidUUID",
  InvalidMUID: "InvalidMUID",

  UUIDNotFound: "UUIDNotFound",
  MUIDNotFound: "MUIDNotFound",

  UUIDAlreadyClaimed: "UUIDAlreadyClaimed",
  AlreadyRegistered: "AlreadyRegistered",

  MUIDMismatch: "MUIDMismatch",

  InvalidMetaData: "InvalidMetaData",
  InvalidVoteMethod: "InvalidVoteMethod",
  VotingInactive: "VotingInactive",

  SignatureInvalid: "SignatureInvalid",
  SignatureExpired: "SignatureExpired",
  SignatureFailure: "SignatureFailure",

  InvalidStatusCode: "InvalidStatusCode",
};

export type APIErrorCode = (typeof APIErrorCodes)[keyof typeof APIErrorCodes];

export type APIError = {
  code: APIErrorCode;
  message: string;
  httpStatus: number;
  timestamp: string;
  endpoint: {
    method: string;
    path: string;
  };
};
