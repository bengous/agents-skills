// Sync stdin requires fs.readFileSync — Effect FileSystem is async-only
import { readFileSync } from "node:fs";
import { Effect } from "effect";
import { StdinError } from "./errors.ts";

const readStdin: Effect.Effect<string, StdinError> = Effect.try({
  try: () => readFileSync(0, "utf-8"),
  catch: (e) => new StdinError({ message: `Failed to read stdin: ${e}` }),
});

export { readStdin };
