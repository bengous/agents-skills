export type NonEmptyPaths = readonly [first: string, ...rest: string[]];

export function nonEmptyPaths(paths: readonly string[]): NonEmptyPaths | null {
  const [first, ...rest] = paths;
  return first === undefined ? null : [first, ...rest];
}

export const DEFAULT_IGNORES = ["node_modules", "dist"] satisfies readonly string[];
