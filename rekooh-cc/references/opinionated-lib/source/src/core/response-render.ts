/**
 * Renders typed response objects into Claude-compatible output.
 *
 * Each response family maps to a specific stdout JSON shape,
 * stderr message, and exit code per the official hooks reference.
 */

import type { EventName } from "./events.ts";
import { HookExit } from "./exit-codes.ts";
import type {
  ContinueStopResponse,
  PermissionRequestResponse,
  PreToolUseResponse,
  SideEffectResponse,
  TopLevelDecisionResponse,
  WorktreeResponse,
} from "./response-families.ts";
import type { ResponseForEvent } from "./response-map.ts";

interface HookOutput {
  readonly exitCode: number;
  readonly stdout: string | null;
  readonly stderr: string | null;
}

// -- Family renderers -------------------------------------------------------

function renderPreToolUse(response: PreToolUseResponse): HookOutput {
  switch (response._tag) {
    case "Allow":
      return { exitCode: HookExit.Allow, stdout: null, stderr: null };
    case "Deny":
      return {
        exitCode: HookExit.Block,
        stdout: null,
        stderr: response.reason,
      };
    case "Ask":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            permissionDecision: "ask",
            additionalContext: response.reason,
          },
        }),
        stderr: null,
      };
    case "AllowWithInput":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            permissionDecision: "allow",
            updatedInput: response.updatedInput,
            additionalContext: response.additionalContext,
          },
        }),
        stderr: null,
      };
  }
}

function renderPermissionRequest(response: PermissionRequestResponse): HookOutput {
  switch (response._tag) {
    case "Allow":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            decision: { behavior: "allow" },
          },
        }),
        stderr: null,
      };
    case "Deny":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            decision: { behavior: "deny" },
          },
        }),
        stderr: null,
      };
    case "AllowWithDecision":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            decision: {
              behavior: "allow",
              updatedInput: response.updatedInput,
              updatedPermissions: response.updatedPermissions,
              message: response.message,
              interrupt: response.interrupt,
            },
          },
        }),
        stderr: null,
      };
  }
}

function renderTopLevelDecision(response: TopLevelDecisionResponse): HookOutput {
  switch (response._tag) {
    case "Proceed":
      return { exitCode: HookExit.Allow, stdout: null, stderr: null };
    case "Block":
      return {
        exitCode: HookExit.Block,
        stdout: null,
        stderr: response.reason,
      };
  }
}

function renderContinueStop(response: ContinueStopResponse): HookOutput {
  switch (response._tag) {
    case "AllowStop":
      return { exitCode: HookExit.Allow, stdout: null, stderr: null };
    case "PreventStop":
      return {
        exitCode: HookExit.Block,
        stdout: null,
        stderr: response.reason,
      };
  }
}

function renderWorktree(response: WorktreeResponse): HookOutput {
  return {
    exitCode: HookExit.Allow,
    stdout: response.path,
    stderr: null,
  };
}

function renderSideEffect(response: SideEffectResponse): HookOutput {
  switch (response._tag) {
    case "Done":
      return { exitCode: HookExit.Allow, stdout: null, stderr: null };
    case "Context":
      return {
        exitCode: HookExit.Allow,
        stdout: JSON.stringify({
          hookSpecificOutput: {
            additionalContext: response.additionalContext,
          },
        }),
        stderr: null,
      };
  }
}

// -- Event-to-renderer dispatch ---------------------------------------------

type RenderFamily = "preToolUse" | "permissionRequest" | "topLevel" | "continueStop" | "worktree" | "sideEffect";

const eventRenderFamily: Record<EventName, RenderFamily> = {
  PreToolUse: "preToolUse",
  PermissionRequest: "permissionRequest",
  PostToolUse: "topLevel",
  PostToolUseFailure: "topLevel",
  UserPromptSubmit: "topLevel",
  ConfigChange: "topLevel",
  Stop: "continueStop",
  SubagentStop: "continueStop",
  TeammateIdle: "continueStop",
  TaskCompleted: "continueStop",
  WorktreeCreate: "worktree",
  SessionStart: "sideEffect",
  SessionEnd: "sideEffect",
  InstructionsLoaded: "sideEffect",
  SubagentStart: "sideEffect",
  Notification: "sideEffect",
  WorktreeRemove: "sideEffect",
  PreCompact: "sideEffect",
};

function renderResponse<E extends EventName>(event: E, response: ResponseForEvent[E]): HookOutput {
  const family = eventRenderFamily[event];
  switch (family) {
    case "preToolUse":
      return renderPreToolUse(response as PreToolUseResponse);
    case "permissionRequest":
      return renderPermissionRequest(response as PermissionRequestResponse);
    case "topLevel":
      return renderTopLevelDecision(response as TopLevelDecisionResponse);
    case "continueStop":
      return renderContinueStop(response as ContinueStopResponse);
    case "worktree":
      return renderWorktree(response as WorktreeResponse);
    case "sideEffect":
      return renderSideEffect(response as SideEffectResponse);
  }
}

export {
  type HookOutput,
  renderContinueStop,
  renderPermissionRequest,
  renderPreToolUse,
  renderResponse,
  renderSideEffect,
  renderTopLevelDecision,
  renderWorktree,
};
