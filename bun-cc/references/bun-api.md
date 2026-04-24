# Bun API Reference

Runtime APIs that matter most for Bun CLIs and small Bun-native tools.

## Table of Contents

- [Bun Shell](#bun-shell)
- [Spawn](#spawn)
- [File I/O](#file-io)
- [Archive](#archive)
- [Utilities](#utilities)

---

## Bun Shell

Import `$` from `bun` for shell commands with automatic escaping of interpolated
values.

```typescript
import { $ } from "bun";

const branch = await $`git rev-parse --abbrev-ref HEAD`.text();
const args = ["log", "--oneline", "-3"];
await $`git ${args}`;
```

### Output and errors

| Pattern | Use |
|---|---|
| `await $\`cmd\`` | Run and inherit output; throws on non-zero exit |
| `await $\`cmd\`.text()` | Capture stdout as a string; does not trim |
| `await $\`cmd\`.json<T>()` | Parse stdout JSON |
| `for await (const line of $\`cmd\`.lines())` | Stream stdout by line |
| `await $\`cmd\`.quiet()` | Capture stdout/stderr as Buffers |
| `await $\`cmd\`.nothrow().quiet()` | Inspect `exitCode` manually |

```typescript
const { stdout, stderr, exitCode } = await $`git status --short`
  .nothrow()
  .quiet();

if (exitCode !== 0) {
  throw new Error(`git failed: ${stderr.toString()}`);
}

console.log(stdout.toString().trim());
```

Use `$.nothrow()` / `$.throws(true)` only for deliberate global policy. For most
scripts, prefer command-local `.nothrow()` so failures stay visible.

### Security boundary

Interpolated values are escaped by Bun Shell:

```typescript
const path = "file.txt; rm -rf /";
await $`cat ${path}`; // treated as one escaped argument
```

That protection does not extend into a shell you explicitly invoke:

```typescript
const script = `cat ${path}`;
await $`bash -c ${script}`; // bash interprets script
```

Avoid `bash -c` for user-controlled strings. If you need raw shell scripts, build
the script from constants and pass untrusted data as separate args or environment.

---

## Spawn

Use `Bun.spawn()` when you need streaming, long-running processes, or precise stdio.
Use `Bun.spawnSync()` only when blocking is acceptable.

```typescript
const proc = Bun.spawn(["git", "status", "--short"], {
  stdout: "pipe",
  stderr: "pipe",
});

const [stdout, stderr, exitCode] = await Promise.all([
  new Response(proc.stdout).text(),
  new Response(proc.stderr).text(),
  proc.exited,
]);

if (exitCode !== 0) {
  throw new Error(`git failed: ${stderr}`);
}
console.log(stdout);
```

```typescript
const result = Bun.spawnSync(["/usr/bin/git", "status", "--short"]);
if (!result.success) {
  throw new Error(result.stderr.toString());
}
```

Prefer absolute paths or `Bun.which()` for binaries where PATH spoofing matters.

---

## File I/O

`Bun.file()` is lazy; it creates a file reference and reads only when a method is
called.

```typescript
const file = Bun.file("data.json");

if (!(await file.exists())) {
  throw new Error("data.json not found");
}

const data = await file.json();
```

Important details:

- `exists()` returns `Promise<boolean>`; an un-awaited promise is always truthy.
- A `BunFile` can point to a missing file; `size` may be `0` before existence is checked.
- `Bun.write()` accepts string paths, `file://` URLs, `BunFile`, `Blob`, typed arrays,
  `ArrayBuffer`, `Response`, and other supported binary data.
- Use `node:fs/promises` for directories (`mkdir`, `readdir`, recursive `rm`).

```typescript
await Bun.write("output.json", JSON.stringify(data, null, 2));
await Bun.write("copy.txt", Bun.file("source.txt"));
await Bun.write("page.html", await fetch("https://example.com"));
```

Incremental writes:

```typescript
const writer = Bun.file("large.log").writer();
writer.write("line 1\n");
writer.write("line 2\n");
writer.end();
```

---

## Archive

`Bun.Archive` works with tar and tar.gz data. It can create archives from an object
map, extract archives to disk, or inspect regular files without extracting.

```typescript
const archive = new Bun.Archive({
  "README.md": "# Project",
  "src/index.ts": "console.log('hi')",
});

await Bun.write("bundle.tar", archive);
await Bun.write("bundle.tar.gz", new Bun.Archive({
  "README.md": "# Project",
  "src/index.ts": "console.log('hi')",
}, { compress: "gzip" }));
```

Prefer a plain object when creating archives:

```typescript
const compressed = new Bun.Archive(
  { "src/index.ts": await Bun.file("src/index.ts").text() },
  { compress: "gzip", level: 9 },
);
await Bun.write("src.tar.gz", compressed);
```

Extracting:

```typescript
const response = await fetch(url);
if (!response.ok) {
  throw new Error(`download failed: ${response.status}`);
}

const archive = new Bun.Archive(await response.blob());
const count = await archive.extract("./output", {
  glob: ["**", "!node_modules/**"],
});
console.log(`Extracted ${count} entries`);
```

Notes:

- `extract()` creates the target directory and overwrites existing files.
- Negative glob patterns require a positive pattern such as `**` first.
- `files()` returns regular files only and loads contents into memory.

---

## Utilities

```typescript
const git = Bun.which("git");
if (!git) throw new Error("git not found");

const args = Bun.argv.slice(2);
await Bun.sleep(100);

if (import.meta.main) {
  // CLI entrypoint
}
```

Use `process.versions.bun` when runtime detection is needed and `@types/bun` may
not be installed.
