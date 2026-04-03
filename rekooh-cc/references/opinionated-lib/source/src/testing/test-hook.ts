/**
 * Subprocess test runner for hook files.
 *
 * Spawns the hook as a child process with JSON stdin,
 * captures stdout/stderr/exitCode for assertions.
 */

interface HookResult {
  readonly exitCode: number;
  readonly stdout: string;
  readonly stderr: string;
  json: () => unknown | null;
}

async function spawnHook(hookPath: string, stdinContent: string, env?: Record<string, string>): Promise<HookResult> {
  const proc = Bun.spawn(["bun", hookPath], {
    stdin: new Blob([stdinContent]),
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, ...env },
  });

  const [stdout, stderr, exitCode] = await Promise.all([proc.stdout.text(), proc.stderr.text(), proc.exited]);

  return {
    exitCode,
    stdout: stdout.trim(),
    stderr: stderr.trim(),
    json: () => (stdout.trim() !== "" ? JSON.parse(stdout.trim()) : null),
  };
}

function testHook(hookPath: string) {
  return {
    run(input: Record<string, unknown>, env?: Record<string, string>): Promise<HookResult> {
      return spawnHook(hookPath, JSON.stringify(input), env);
    },

    runRaw(rawStdin: string, env?: Record<string, string>): Promise<HookResult> {
      return spawnHook(hookPath, rawStdin, env);
    },
  };
}

export { type HookResult, testHook };
