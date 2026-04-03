import { Effect, Schema } from "effect";
import { DecodeError } from "./errors.ts";
import { EventSchemas, type InputForEvent } from "./event-inputs.ts";
import type { EventName } from "./events.ts";

/**
 * Parse raw JSON string into unknown object.
 */
const parseJson = (raw: string): Effect.Effect<unknown, DecodeError> =>
  Effect.try({
    try: () => JSON.parse(raw) as unknown,
    catch: (e) => new DecodeError({ message: `Invalid JSON: ${e}`, cause: e }),
  });

/**
 * Decode a parsed JSON object into a typed event input.
 * Uses the event name to select the right schema.
 */
function decodeEvent<E extends EventName>(
  event: E,
  raw: unknown,
): Effect.Effect<InputForEvent[E] & { readonly _raw: Record<string, unknown> }, DecodeError> {
  const schema = EventSchemas[event] as unknown as Schema.Schema<InputForEvent[E]>;
  return Effect.try({
    try: () =>
      Object.assign(Schema.decodeUnknownSync(schema)(raw), {
        _raw: raw as Record<string, unknown>,
      }) as InputForEvent[E] & {
        readonly _raw: Record<string, unknown>;
      },
    catch: (e) =>
      new DecodeError({
        message: `Failed to decode ${event} input: ${e}`,
        cause: e,
      }),
  });
}

export { decodeEvent, parseJson };
