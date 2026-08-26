import { stdin } from "node:process";

export function withRawMode<T>(body: () => T): T {
  const wasRaw = stdin.isRaw;
  stdin.setRawMode(true);
  try {
    return body();
  } finally {
    stdin.setRawMode(wasRaw);
  }
}
