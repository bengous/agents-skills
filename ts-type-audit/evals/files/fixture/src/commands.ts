export type Command =
  | { kind: "build"; target: string }
  | { kind: "test"; filter?: string }
  | { kind: "lint" };

const COMMAND_NAMES = ["build", "test", "lint"] as const;
type CommandName = (typeof COMMAND_NAMES)[number];

export function parseCommand(argv: readonly string[]): Command {
  const head = argv[0];
  if (head === undefined || !COMMAND_NAMES.includes(head as CommandName)) {
    throw new Error(`unknown command: ${head ?? "<none>"}`);
  }
  const name = head as CommandName;
  if (name === "build") return { kind: "build", target: argv[1] ?? "." };
  if (name === "test") return { kind: "test", filter: argv[1] };
  return { kind: "lint" };
}

export function run(command: Command): string {
  switch (command.kind) {
    case "build":
      return `building ${command.target}`;
    case "test":
      return `testing ${command.filter ?? "*"}`;
    default:
      return "linting";
  }
}
