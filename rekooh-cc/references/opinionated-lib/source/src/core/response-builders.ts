/**
 * Type-safe response builder namespaces.
 *
 * Each event family gets its own set of builders. The defineHook
 * API passes the correct builder namespace to check callbacks.
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

// -- Builder namespaces -----------------------------------------------------

const preToolUse = {
  allow: (): PreToolUseResponse => ({ _tag: "Allow" }),
  deny: (reason: string): PreToolUseResponse => ({ _tag: "Deny", reason }),
  ask: (reason?: string): PreToolUseResponse => (reason !== undefined ? { _tag: "Ask", reason } : { _tag: "Ask" }),
  allowWithInput: (updatedInput: Record<string, unknown>, additionalContext?: string): PreToolUseResponse =>
    additionalContext !== undefined
      ? { _tag: "AllowWithInput", updatedInput, additionalContext }
      : { _tag: "AllowWithInput", updatedInput },
};

const permissionRequest = {
  allow: (): PermissionRequestResponse => ({ _tag: "Allow" }),
  deny: (): PermissionRequestResponse => ({ _tag: "Deny" }),
  allowWithDecision: (opts: {
    updatedInput?: Record<string, unknown>;
    updatedPermissions?: ReadonlyArray<unknown>;
    message?: string;
    interrupt?: boolean;
  }): PermissionRequestResponse => ({ _tag: "AllowWithDecision", ...opts }),
};

const topLevel = {
  proceed: (): TopLevelDecisionResponse => ({ _tag: "Proceed" }),
  block: (reason: string, additionalContext?: string): TopLevelDecisionResponse =>
    additionalContext !== undefined ? { _tag: "Block", reason, additionalContext } : { _tag: "Block", reason },
};

const continueStop = {
  allowStop: (): ContinueStopResponse => ({ _tag: "AllowStop" }),
  preventStop: (reason: string): ContinueStopResponse => ({
    _tag: "PreventStop",
    reason,
  }),
};

const worktree = {
  path: (absolutePath: string): WorktreeResponse => ({
    _tag: "Path",
    path: absolutePath,
  }),
};

const sideEffect = {
  done: (): SideEffectResponse => ({ _tag: "Done" }),
  context: (additionalContext: string): SideEffectResponse => ({
    _tag: "Context",
    additionalContext,
  }),
};

// -- Event-to-builder mapping -----------------------------------------------

type RespondersForEvent = {
  PreToolUse: typeof preToolUse;
  PermissionRequest: typeof permissionRequest;
  PostToolUse: typeof topLevel;
  PostToolUseFailure: typeof topLevel;
  UserPromptSubmit: typeof topLevel;
  ConfigChange: typeof topLevel;
  Stop: typeof continueStop;
  SubagentStop: typeof continueStop;
  TeammateIdle: typeof continueStop;
  TaskCompleted: typeof continueStop;
  WorktreeCreate: typeof worktree;
  SessionStart: typeof sideEffect;
  SessionEnd: typeof sideEffect;
  InstructionsLoaded: typeof sideEffect;
  SubagentStart: typeof sideEffect;
  Notification: typeof sideEffect;
  WorktreeRemove: typeof sideEffect;
  PreCompact: typeof sideEffect;
};

const respondersForEvent: Record<EventName, unknown> = {
  PreToolUse: preToolUse,
  PermissionRequest: permissionRequest,
  PostToolUse: topLevel,
  PostToolUseFailure: topLevel,
  UserPromptSubmit: topLevel,
  ConfigChange: topLevel,
  Stop: continueStop,
  SubagentStop: continueStop,
  TeammateIdle: continueStop,
  TaskCompleted: continueStop,
  WorktreeCreate: worktree,
  SessionStart: sideEffect,
  SessionEnd: sideEffect,
  InstructionsLoaded: sideEffect,
  SubagentStart: sideEffect,
  Notification: sideEffect,
  WorktreeRemove: sideEffect,
  PreCompact: sideEffect,
};

function getResponders<E extends EventName>(event: E): RespondersForEvent[E] {
  return respondersForEvent[event] as RespondersForEvent[E];
}

export {
  type RespondersForEvent,
  continueStop,
  getResponders,
  permissionRequest,
  preToolUse,
  sideEffect,
  topLevel,
  worktree,
};
