/**
 * Strip string literals and heredocs from shell commands.
 *
 * Used by guard presets to avoid false positives when destructive
 * commands appear inside commit messages or echo statements.
 */

function stripStringLiterals(cmd: string): string {
  // Strip heredocs: <<'EOF' ... EOF, <<"EOF" ... EOF, <<EOF ... EOF
  let stripped = cmd.replace(/<<-?\s*'?(\w+)'?.*?\n[\s\S]*?\n\s*\1/g, "");
  // Strip double-quoted strings (non-greedy, respecting escapes)
  stripped = stripped.replace(/"(?:[^"\\]|\\.)*"/g, '""');
  // Strip single-quoted strings (no escapes in single quotes)
  stripped = stripped.replace(/'[^']*'/g, "''");
  return stripped;
}

export { stripStringLiterals };
