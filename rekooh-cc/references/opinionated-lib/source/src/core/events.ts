import { Schema } from "effect";

const eventNames = [
  "SessionStart",
  "InstructionsLoaded",
  "UserPromptSubmit",
  "PreToolUse",
  "PermissionRequest",
  "PostToolUse",
  "PostToolUseFailure",
  "Notification",
  "SubagentStart",
  "SubagentStop",
  "Stop",
  "TeammateIdle",
  "TaskCompleted",
  "ConfigChange",
  "WorktreeCreate",
  "WorktreeRemove",
  "PreCompact",
  "SessionEnd",
] as const;

const EventName = Schema.Literal(...eventNames);
type EventName = typeof EventName.Type;

export { EventName, eventNames };
