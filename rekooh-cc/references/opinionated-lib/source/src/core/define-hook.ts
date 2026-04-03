/**
 * defineHook — the single public entry point for authoring command hooks.
 *
 * When the module containing defineHook executes, it immediately reads
 * stdin, decodes the event, runs the check callback, renders the response,
 * and exits with the appropriate code.
 */

import { Effect, pipe } from "effect";
import { decodeEvent, parseJson } from "./decode.ts";
import { CheckError } from "./errors.ts";
import type { InputForEvent } from "./event-inputs.ts";
import type { EventName } from "./events.ts";
import { HookExit } from "./exit-codes.ts";
import { allowOutput, applyPolicy, type FailurePolicy } from "./failure-policy.ts";
import { getResponders, type RespondersForEvent } from "./response-builders.ts";
import type { ResponseForEvent } from "./response-map.ts";
import { type HookOutput, renderResponse } from "./response-render.ts";
import { readStdin } from "./stdin.ts";

// -- Matcher config ---------------------------------------------------------

/**
 * Events that support matcher filtering and the field they match against.
 */
const matcherField: Partial<Record<EventName, string>> = {
  PreToolUse: "tool_name",
  PostToolUse: "tool_name",
  PostToolUseFailure: "tool_name",
  PermissionRequest: "tool_name",
  SessionStart: "source",
  SessionEnd: "reason",
  Notification: "notification_type",
  SubagentStart: "agent_type",
  SubagentStop: "agent_type",
  PreCompact: "trigger",
  ConfigChange: "source",
};

// -- Hook config type -------------------------------------------------------

type CheckFn<E extends EventName> = (
  input: InputForEvent[E] & { readonly _raw: Record<string, unknown> },
  respond: RespondersForEvent[E],
) => ResponseForEvent[E] | Effect.Effect<ResponseForEvent[E], CheckError> | Promise<ResponseForEvent[E]>;

interface HookConfig<E extends EventName> {
  matcher?: string | RegExp;
  check: CheckFn<E>;
  failurePolicy?: FailurePolicy;
}

// -- Core pipeline ----------------------------------------------------------

function defineHook<E extends EventName>(event: E, config: HookConfig<E>): void {
  const respond = getResponders(event);
  const policy = config.failurePolicy ?? "fail-open";
  const compiledMatcher =
    config.matcher instanceof RegExp
      ? config.matcher
      : config.matcher !== undefined
        ? new RegExp(`^${config.matcher}$`)
        : undefined;

  const pipeline = pipe(
    readStdin,
    Effect.flatMap(parseJson),
    Effect.flatMap((raw) => {
      // Matcher short-circuit: check matcher field before full decode
      if (compiledMatcher !== undefined) {
        const field = matcherField[event];
        if (field !== undefined) {
          const obj = raw as Record<string, unknown>;
          const value = obj[field];
          if (typeof value === "string" && !compiledMatcher.test(value)) {
            return Effect.succeed(allowOutput as HookOutput);
          }
        }
      }
      // Full decode + check
      return pipe(
        decodeEvent(event, raw),
        Effect.flatMap((input) => {
          const result = config.check(input, respond);
          if (Effect.isEffect(result)) {
            return result as Effect.Effect<ResponseForEvent[E], CheckError>;
          }
          if (result instanceof Promise) {
            return Effect.tryPromise({
              try: () => result,
              catch: (e) =>
                new CheckError({
                  message: `Check callback promise rejected: ${e}`,
                  cause: e,
                }),
            });
          }
          return Effect.succeed(result);
        }),
        Effect.map((response) => renderResponse(event, response)),
      );
    }),
    applyPolicy(policy),
  );

  const fatalExitCode = policy === "fail-closed" ? HookExit.Block : HookExit.Error;

  Effect.runPromise(pipeline).then(
    ({ stdout, stderr, exitCode }) => {
      if (stdout !== null) process.stdout.write(`${stdout}\n`);
      if (stderr !== null) process.stderr.write(`${stderr}\n`);
      process.exit(exitCode);
    },
    (error) => {
      process.stderr.write(`HOOK FATAL: ${error}\n`);
      process.exit(fatalExitCode);
    },
  );
}

export { type CheckFn, type HookConfig, defineHook };
