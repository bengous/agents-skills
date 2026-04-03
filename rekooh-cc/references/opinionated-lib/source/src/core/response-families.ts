/**
 * Response type families matching Claude's real decision contracts.
 *
 * Each event maps to exactly one response family. The framework
 * renders these into Claude-compatible stdout/stderr/exitCode.
 */

// Family 1: PreToolUse — hookSpecificOutput.permissionDecision
type PreToolUseResponse =
  | { readonly _tag: "Allow" }
  | { readonly _tag: "Deny"; readonly reason: string }
  | { readonly _tag: "Ask"; readonly reason?: string }
  | {
      readonly _tag: "AllowWithInput";
      readonly updatedInput: Record<string, unknown>;
      readonly additionalContext?: string;
    };

// Family 2: PermissionRequest — hookSpecificOutput.decision.behavior
type PermissionRequestResponse =
  | { readonly _tag: "Allow" }
  | { readonly _tag: "Deny" }
  | {
      readonly _tag: "AllowWithDecision";
      readonly updatedInput?: Record<string, unknown>;
      readonly updatedPermissions?: ReadonlyArray<unknown>;
      readonly message?: string;
      readonly interrupt?: boolean;
    };

// Family 3: Top-level decision — decision + reason (PostToolUse, UserPromptSubmit, etc.)
type TopLevelDecisionResponse =
  | { readonly _tag: "Proceed" }
  | {
      readonly _tag: "Block";
      readonly reason: string;
      readonly additionalContext?: string;
    };

// Family 4: Continue/Stop — Stop, SubagentStop, TeammateIdle, TaskCompleted
type ContinueStopResponse = { readonly _tag: "AllowStop" } | { readonly _tag: "PreventStop"; readonly reason: string };

// Family 5: WorktreeCreate — stdout absolute path
type WorktreeResponse = { readonly _tag: "Path"; readonly path: string };

// Family 6: Side-effect-only — SessionStart, SessionEnd, InstructionsLoaded, etc.
type SideEffectResponse = { readonly _tag: "Done" } | { readonly _tag: "Context"; readonly additionalContext: string };

export type {
  ContinueStopResponse,
  PermissionRequestResponse,
  PreToolUseResponse,
  SideEffectResponse,
  TopLevelDecisionResponse,
  WorktreeResponse,
};
