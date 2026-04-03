import { Data } from "effect";

class StdinError extends Data.TaggedError("StdinError")<{
  readonly message: string;
}> {}

class DecodeError extends Data.TaggedError("DecodeError")<{
  readonly message: string;
  readonly cause?: unknown;
}> {}

class CheckError extends Data.TaggedError("CheckError")<{
  readonly message: string;
  readonly cause?: unknown;
}> {}

export { CheckError, DecodeError, StdinError };
