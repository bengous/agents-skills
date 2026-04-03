// Interactive prompts for CLI

// ============================================================================
// SIDE-EFFECTING FUNCTIONS
// ============================================================================

export async function prompt(question: string): Promise<string> {
  process.stdout.write(question);
  for await (const line of console) return line.trim();
  return "";
}

// Note: No input masking in this demo. For production, use a TTY raw mode library.
export async function promptPassword(question: string): Promise<string> {
  return prompt(question);
}

export async function confirm(question: string): Promise<boolean> {
  process.stdout.write(`${question} [Y/n] `);
  for await (const line of console) return line.trim().toLowerCase() !== "n";
  return false;
}
