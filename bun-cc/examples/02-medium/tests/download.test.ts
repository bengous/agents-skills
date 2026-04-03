import { describe, expect, test } from "bun:test";
import { formatSize } from "../src/download";

describe("formatSize", () => {
  test.each([
    [0, "0 B"],
    [512, "512 B"],
    [1024, "1.0 KB"],
    [1536, "1.5 KB"],
    [1048576, "1.0 MB"],
    [1572864, "1.5 MB"],
  ])("formatSize(%d) -> %s", (bytes, expected) => {
    expect(formatSize(bytes)).toBe(expected);
  });
});
