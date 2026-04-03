import { Effect } from "effect";
import type { CheckError, DecodeError, StdinError } from "./errors.ts";
import { HookExit } from "./exit-codes.ts";
import type { HookOutput } from "./response-render.ts";

type FailurePolicy = "fail-open" | "fail-closed";

type PipelineError = StdinError | DecodeError | CheckError;

const allowOutput: HookOutput = {
  exitCode: HookExit.Allow,
  stdout: null,
  stderr: null,
};

const blockOutput = (message: string): HookOutput => ({
  exitCode: HookExit.Block,
  stdout: null,
  stderr: message,
});

function applyPolicy(policy: FailurePolicy) {
  return (effect: Effect.Effect<HookOutput, PipelineError>): Effect.Effect<HookOutput, never> =>
    policy === "fail-open"
      ? Effect.catchAll(effect, (e) => {
          process.stderr.write(`HOOK WARNING (fail-open): ${e.message}\n`);
          return Effect.succeed(allowOutput);
        })
      : Effect.catchAll(effect, (e) => {
          return Effect.succeed(blockOutput(`HOOK ERROR (fail-closed): ${e.message}`));
        });
}

export { type FailurePolicy, type PipelineError, allowOutput, applyPolicy };
