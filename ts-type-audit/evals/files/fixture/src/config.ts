import { readFileSync } from "node:fs";

export interface Config {
  port: number;
  dataDir: string;
}

export function loadConfig(path: string): Config {
  const raw = JSON.parse(readFileSync(path, "utf8")) as Config;
  const port = Number(process.env["PORT"] ?? raw.port);
  return { port, dataDir: raw.dataDir };
}
