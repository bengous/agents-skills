import { Schema } from "effect";
import { EventName } from "./events.ts";

/**
 * Fields present on every hook event's stdin JSON.
 * Extracted from the official hooks reference.
 */
const CommonInput = Schema.Struct({
  session_id: Schema.String,
  transcript_path: Schema.String,
  cwd: Schema.String,
  permission_mode: Schema.String,
  hook_event_name: EventName,
  agent_id: Schema.optional(Schema.String),
  agent_type: Schema.optional(Schema.String),
});

type CommonInput = typeof CommonInput.Type;

export { CommonInput };
