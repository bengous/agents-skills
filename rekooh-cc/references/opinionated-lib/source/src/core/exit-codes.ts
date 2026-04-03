const HookExit = {
  Allow: 0,
  Error: 1,
  Block: 2,
} as const;

type HookExit = (typeof HookExit)[keyof typeof HookExit];

export { HookExit };
