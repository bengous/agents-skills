/**
 * Maps each event name to its response family type.
 * Used by defineHook to constrain check callback return types.
 */

import type { EventName } from "./events.ts";
import type {
  ContinueStopResponse,
  PermissionRequestResponse,
  PreToolUseResponse,
  SideEffectResponse,
  TopLevelDecisionResponse,
  WorktreeResponse,
} from "./response-families.ts";

type ResponseForEvent = {
  PreToolUse: PreToolUseResponse;
  PermissionRequest: PermissionRequestResponse;
  PostToolUse: TopLevelDecisionResponse;
  PostToolUseFailure: TopLevelDecisionResponse;
  UserPromptSubmit: TopLevelDecisionResponse;
  ConfigChange: TopLevelDecisionResponse;
  Stop: ContinueStopResponse;
  SubagentStop: ContinueStopResponse;
  TeammateIdle: ContinueStopResponse;
  TaskCompleted: ContinueStopResponse;
  WorktreeCreate: WorktreeResponse;
  SessionStart: SideEffectResponse;
  SessionEnd: SideEffectResponse;
  InstructionsLoaded: SideEffectResponse;
  SubagentStart: SideEffectResponse;
  Notification: SideEffectResponse;
  WorktreeRemove: SideEffectResponse;
  PreCompact: SideEffectResponse;
};

// Compile-time check: every EventName has a mapping
type _check = ResponseForEvent extends Record<EventName, unknown> ? true : never;
const _exhaustive: _check = true;
void _exhaustive;

export type { ResponseForEvent };
