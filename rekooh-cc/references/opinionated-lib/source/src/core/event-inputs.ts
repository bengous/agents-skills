/**
 * Effect Schemas for all 18 command-capable hook event inputs.
 *
 * Each schema extends CommonInput with event-specific fields.
 * Unknown fields are preserved via _raw on the decoded type.
 *
 * Source of truth: official hooks reference page.
 */

import { Schema } from "effect";
import { CommonInput } from "./common-input.ts";
import type { EventName } from "./events.ts";

// -- Shared field schemas ---------------------------------------------------

const ToolInput = Schema.Record({
  key: Schema.String,
  value: Schema.Unknown,
});

// -- Event-specific schemas -------------------------------------------------

const SessionStartInput = Schema.Struct({
  ...CommonInput.fields,
  source: Schema.String,
  model: Schema.optional(Schema.String),
});

const InstructionsLoadedInput = Schema.Struct({
  ...CommonInput.fields,
  file_path: Schema.String,
  memory_type: Schema.String,
  load_reason: Schema.String,
  globs: Schema.optional(Schema.String),
  trigger_file_path: Schema.optional(Schema.String),
  parent_file_path: Schema.optional(Schema.String),
});

const UserPromptSubmitInput = Schema.Struct({
  ...CommonInput.fields,
  prompt: Schema.String,
});

const PreToolUseInput = Schema.Struct({
  ...CommonInput.fields,
  tool_name: Schema.String,
  tool_input: ToolInput,
  tool_use_id: Schema.optional(Schema.String),
});

const PermissionRequestInput = Schema.Struct({
  ...CommonInput.fields,
  tool_name: Schema.String,
  tool_input: ToolInput,
  permission_suggestions: Schema.optional(Schema.Unknown),
});

const PostToolUseInput = Schema.Struct({
  ...CommonInput.fields,
  tool_name: Schema.String,
  tool_input: ToolInput,
  tool_use_id: Schema.optional(Schema.String),
  tool_response: Schema.optional(ToolInput),
});

const PostToolUseFailureInput = Schema.Struct({
  ...CommonInput.fields,
  tool_name: Schema.String,
  tool_input: ToolInput,
  tool_use_id: Schema.optional(Schema.String),
  error: Schema.String,
  is_interrupt: Schema.optional(Schema.Boolean),
});

const NotificationInput = Schema.Struct({
  ...CommonInput.fields,
  message: Schema.String,
  title: Schema.optional(Schema.String),
  notification_type: Schema.String,
});

const SubagentStartInput = Schema.Struct({
  ...CommonInput.fields,
  agent_id: Schema.String,
  agent_type: Schema.String,
});

const SubagentStopInput = Schema.Struct({
  ...CommonInput.fields,
  stop_hook_active: Schema.Boolean,
  agent_id: Schema.String,
  agent_type: Schema.String,
  agent_transcript_path: Schema.String,
  last_assistant_message: Schema.optional(Schema.String),
});

const StopInput = Schema.Struct({
  ...CommonInput.fields,
  stop_hook_active: Schema.Boolean,
  last_assistant_message: Schema.optional(Schema.String),
});

const TeammateIdleInput = Schema.Struct({
  ...CommonInput.fields,
  teammate_name: Schema.String,
  team_name: Schema.String,
});

const TaskCompletedInput = Schema.Struct({
  ...CommonInput.fields,
  task_id: Schema.String,
  task_subject: Schema.String,
  task_description: Schema.optional(Schema.String),
  teammate_name: Schema.optional(Schema.String),
  team_name: Schema.optional(Schema.String),
});

const ConfigChangeInput = Schema.Struct({
  ...CommonInput.fields,
  source: Schema.String,
  file_path: Schema.optional(Schema.String),
});

const WorktreeCreateInput = Schema.Struct({
  ...CommonInput.fields,
  name: Schema.String,
});

const WorktreeRemoveInput = Schema.Struct({
  ...CommonInput.fields,
  worktree_path: Schema.String,
});

const PreCompactInput = Schema.Struct({
  ...CommonInput.fields,
  trigger: Schema.String,
  custom_instructions: Schema.optional(Schema.String),
});

const SessionEndInput = Schema.Struct({
  ...CommonInput.fields,
  reason: Schema.String,
});

// -- Schema registry: event name → schema ----------------------------------

const EventSchemas = {
  SessionStart: SessionStartInput,
  InstructionsLoaded: InstructionsLoadedInput,
  UserPromptSubmit: UserPromptSubmitInput,
  PreToolUse: PreToolUseInput,
  PermissionRequest: PermissionRequestInput,
  PostToolUse: PostToolUseInput,
  PostToolUseFailure: PostToolUseFailureInput,
  Notification: NotificationInput,
  SubagentStart: SubagentStartInput,
  SubagentStop: SubagentStopInput,
  Stop: StopInput,
  TeammateIdle: TeammateIdleInput,
  TaskCompleted: TaskCompletedInput,
  ConfigChange: ConfigChangeInput,
  WorktreeCreate: WorktreeCreateInput,
  WorktreeRemove: WorktreeRemoveInput,
  PreCompact: PreCompactInput,
  SessionEnd: SessionEndInput,
} as const satisfies Record<EventName, Schema.Schema.Any>;

// -- Type extraction --------------------------------------------------------

type InputForEvent = {
  [K in EventName]: (typeof EventSchemas)[K]["Type"];
};

type AnyEventInput = InputForEvent[EventName];

export {
  type AnyEventInput,
  ConfigChangeInput,
  EventSchemas,
  type InputForEvent,
  InstructionsLoadedInput,
  NotificationInput,
  PermissionRequestInput,
  PostToolUseFailureInput,
  PostToolUseInput,
  PreCompactInput,
  PreToolUseInput,
  SessionEndInput,
  SessionStartInput,
  StopInput,
  SubagentStartInput,
  SubagentStopInput,
  TaskCompletedInput,
  TeammateIdleInput,
  UserPromptSubmitInput,
  WorktreeCreateInput,
  WorktreeRemoveInput,
};
