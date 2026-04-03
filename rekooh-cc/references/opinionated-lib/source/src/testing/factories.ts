/**
 * Test payload factories for hook testing.
 *
 * createPayload("PreToolUse", { tool_name: "Bash" }) produces a
 * complete, valid payload with sensible defaults for all fields.
 */

import type { InputForEvent } from "../core/event-inputs.ts";
import type { EventName } from "../core/events.ts";

function baseFields(event: EventName) {
  return {
    session_id: "test-session",
    transcript_path: "/tmp/test-transcript.json",
    cwd: "/tmp/test-cwd",
    permission_mode: "default",
    hook_event_name: event,
  } as const;
}

const EVENT_DEFAULTS: Record<EventName, Record<string, unknown>> = {
  SessionStart: { source: "startup", model: "opus" },
  InstructionsLoaded: {
    file_path: "/project/CLAUDE.md",
    memory_type: "Project",
    load_reason: "session_start",
  },
  UserPromptSubmit: { prompt: "test prompt" },
  PreToolUse: { tool_name: "Bash", tool_input: { command: "echo test" } },
  PermissionRequest: {
    tool_name: "Bash",
    tool_input: { command: "echo test" },
  },
  PostToolUse: { tool_name: "Bash", tool_input: { command: "echo test" } },
  PostToolUseFailure: {
    tool_name: "Bash",
    tool_input: { command: "exit 1" },
    error: "Command failed",
  },
  Notification: {
    message: "Test notification",
    notification_type: "permission_prompt",
  },
  SubagentStart: { agent_id: "agent_001", agent_type: "Explore" },
  SubagentStop: {
    stop_hook_active: false,
    agent_id: "agent_001",
    agent_type: "Explore",
    agent_transcript_path: "/tmp/agent-transcript.json",
  },
  Stop: { stop_hook_active: false },
  TeammateIdle: { teammate_name: "worker-1", team_name: "build-team" },
  TaskCompleted: { task_id: "task_001", task_subject: "Test task" },
  ConfigChange: { source: "project_settings" },
  WorktreeCreate: { name: "feature-branch" },
  WorktreeRemove: { worktree_path: "/tmp/worktrees/feature" },
  PreCompact: { trigger: "manual" },
  SessionEnd: { reason: "prompt_input_exit" },
};

function createPayload<E extends EventName>(event: E, overrides?: Partial<InputForEvent[E]>): InputForEvent[E] {
  return {
    ...baseFields(event),
    ...EVENT_DEFAULTS[event],
    ...overrides,
  } as InputForEvent[E];
}

export { createPayload };
