# Bun API Reference

Comprehensive reference for Bun runtime APIs used in CLI script development.

## Table of Contents

- [Bun Shell ($)](#bun-shell-)
- [Bun.spawn / Bun.spawnSync](#bunspawn--bunspawnsync)
- [File I/O](#file-io)
- [Bun.Archive](#bunarchive)
- [Utilities](#utilities)

---

## Bun Shell ($)

The `$` tagged template provides a safe, ergonomic way to run shell commands.

### Import

```typescript
import { $ } from "bun";
```

### Basic Usage

```typescript
// Execute command (outputs to stdout by default)
await $`echo "Hello World!"`;

// Capture output as text
const result = await $`echo "Hello"`.text();

// Multi-line scripts
await $`
  HASH=$(git rev-parse HEAD)
  echo "Building $HASH"
  docker build -t app:$HASH .
`;
```

### Output Methods

| Method | Return Type | Description |
|--------|-------------|-------------|
| `.text()` | `Promise<string>` | Returns stdout as string, auto-calls `.quiet()`. **Does NOT trim** — use `.trim()` for CLI output. **Throws on non-zero exit** (inherits default throw behavior). |
| `.json()` | `Promise<T>` | Parses stdout as JSON |
| `.lines()` | `AsyncIterable<string>` | Async iterator of output lines |
| `.blob()` | `Promise<Blob>` | Returns output as Blob |
| `.quiet()` | `ShellPromise` | Suppresses stdout/stderr printing |

### Control Methods

| Method | Description |
|--------|-------------|
| `.nothrow()` | Don't throw on non-zero exit codes |
| `.throws(bool)` | Control throw behavior (default: `true`) |
| `.env(obj)` | Set environment variables for command |
| `.cwd(path)` | Set working directory for command |

### Return Value Structure

```typescript
const { stdout, stderr, exitCode } = await $`command`.quiet();
// stdout: Buffer
// stderr: Buffer
// exitCode: number
```

### Auto-Escaping (Security)

All interpolated values are automatically escaped, preventing shell injection:

```typescript
const userInput = "file.txt; rm -rf /";
await $`cat ${userInput}`;  // SAFE: treated as single quoted string

// Arrays spread into separate, individually-escaped args
const args = ["log", "--oneline", "-3"];
await $`git ${args}`;  // Same as: git 'log' '--oneline' '-3'

// Slashes and special chars in values are safe
const ref = "refs/ship-backup/feature/test";
await $`git rev-parse --verify ${ref}`;  // Works correctly

// Bypass escaping when needed (use with caution)
await $`echo ${{ raw: '$(date)' }}`;
```

### Error Handling

```typescript
// Default: throws on non-zero exit code
try {
  await $`exit 1`;
} catch (err) {
  console.log(err.exitCode);   // 1
  console.log(err.stdout);     // Buffer
  console.log(err.stderr);     // Buffer
}

// Non-throwing pattern (preferred for scripts)
const { exitCode, stdout, stderr } = await $`may-fail`.nothrow().quiet();
if (exitCode !== 0) {
  console.error(`Command failed: ${stderr.toString()}`);
}
```

### Global Configuration

```typescript
$.nothrow();                         // Disable throwing globally
$.throws(true);                      // Re-enable throwing
$.env({ ...process.env, KEY: "val" }); // Set default environment
$.cwd("/tmp");                       // Set default working directory
```

### Redirection

```typescript
await $`cat < input.txt`;           // stdin from file
await $`echo hi > output.txt`;      // stdout to file
await $`cmd 2> errors.txt`;         // stderr to file
await $`cmd >> file.txt`;           // append stdout
await $`cmd 2>&1`;                  // merge stderr into stdout
await $`cmd &> file.txt`;           // both streams to file

// JavaScript object redirection
const buf = Buffer.alloc(1024);
await $`echo hello > ${buf}`;

const resp = new Response("input data");
await $`cat < ${resp}`.text();
```

### Piping

```typescript
const count = await $`echo "one two three" | wc -w`.text();  // "3\n"

// Pipe with JavaScript objects
const response = new Response("hello world");
await $`cat < ${response} | wc -w`.text();
```

### Environment Variables

```typescript
// Inline for single command
await $`FOO=bar bun -e 'console.log(process.env.FOO)'`;

// With interpolation
const value = "hello";
await $`VAR=${value} printenv VAR`.text();

// Override default env for single command
await $`echo $HOME`.env({ HOME: "/custom" });
```

### Utility Functions

```typescript
// Brace expansion
$.braces("file{1,2,3}.txt");  // ["file1.txt", "file2.txt", "file3.txt"]

// Manual escaping
$.escape('$(cmd) `backtick`');  // "\$(cmd) \`backtick\`"
```

### Built-in Commands (Cross-Platform)

`cd`, `ls`, `rm`, `echo`, `pwd`, `cat`, `touch`, `mkdir`, `which`, `mv`, `exit`, `true`, `false`, `yes`, `seq`, `dirname`, `basename`, `bun`

---

## Bun.spawn / Bun.spawnSync

Lower-level process spawning APIs for more control.

### Function Signatures

```typescript
// Async
Bun.spawn(command: string[], options?: SpawnOptions): Subprocess;
Bun.spawn(options: { cmd: string[] } & SpawnOptions): Subprocess;

// Blocking
Bun.spawnSync(command: string[], options?: SpawnOptions): SyncSubprocess;
Bun.spawnSync(options: { cmd: string[] } & SpawnOptions): SyncSubprocess;
```

### SpawnOptions

| Option | Type | Description |
|--------|------|-------------|
| `cwd` | `string` | Working directory |
| `env` | `Record<string, string>` | Environment variables |
| `stdin` | `"pipe" \| "inherit" \| "ignore" \| Bun.file() \| ...` | Input source |
| `stdout` | `"pipe" \| "inherit" \| "ignore" \| Bun.file()` | Output destination (default: `"pipe"`) |
| `stderr` | `"pipe" \| "inherit" \| "ignore" \| Bun.file()` | Error destination (default: `"inherit"`) |
| `onExit` | `(proc, exitCode, signalCode, error) => void` | Exit callback |
| `timeout` | `number` | Kill after milliseconds |

### Subprocess Interface

```typescript
interface Subprocess {
  readonly pid: number;
  readonly stdin: FileSink | undefined;
  readonly stdout: ReadableStream<Uint8Array> | undefined;
  readonly stderr: ReadableStream<Uint8Array> | undefined;
  readonly exited: Promise<number>;
  readonly exitCode: number | null;
  readonly signalCode: NodeJS.Signals | null;
  readonly killed: boolean;

  kill(signal?: number | NodeJS.Signals): void;
  ref(): void;
  unref(): void;
}
```

### SyncSubprocess Interface

```typescript
interface SyncSubprocess {
  stdout: Buffer;
  stderr: Buffer;
  success: boolean;  // true if exitCode === 0
  exitCode: number;
}
```

### When to Use Which

| API | Use Case |
|-----|----------|
| `$\`cmd\`` | General commands, piping, redirection, safe interpolation |
| `Bun.spawn()` | Async, streaming output, IPC, complex stdin |
| `Bun.spawnSync()` | Blocking, absolute path execution, simple results |

### Examples

**Async with streaming output:**
```typescript
const proc = Bun.spawn(["ls", "-la"]);
const output = await proc.stdout.text();
console.log(output);
```

**Sync with success check:**
```typescript
const result = Bun.spawnSync(["git", "status", "--short"]);
if (result.success) {
  console.log(result.stdout.toString());
}
```

**Absolute path (avoid PATH issues):**
```typescript
// When you need a specific binary, not whatever's in PATH
const result = Bun.spawnSync(["/usr/local/bin/codex", "--version"]);
```

**Piping stdin:**
```typescript
const proc = Bun.spawn(["cat"], { stdin: "pipe" });
proc.stdin.write("Hello, World!");
proc.stdin.end();
const output = await proc.stdout.text();
```

**With timeout:**
```typescript
const proc = Bun.spawn({
  cmd: ["sleep", "100"],
  timeout: 5000,
  killSignal: "SIGKILL",
});
```

---

## File I/O

### Bun.file() - File Reference

Creates a lazy file reference (no disk read until a method is called):

```typescript
const file = Bun.file("foo.txt");           // relative path
const file = Bun.file("/abs/path.txt");     // absolute path
const file = Bun.file(fd);                  // file descriptor
```

### Properties

- `size` - file size in bytes
- `type` - MIME type (default: `text/plain;charset=utf-8`)

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `text()` | `Promise<string>` | Read as string |
| `json()` | `Promise<any>` | Parse as JSON |
| `bytes()` | `Promise<Uint8Array>` | Read as bytes |
| `arrayBuffer()` | `Promise<ArrayBuffer>` | Read as ArrayBuffer |
| `stream()` | `ReadableStream` | Get readable stream |
| `exists()` | `Promise<boolean>` | Check existence |
| `delete()` | `Promise<void>` | Delete the file |
| `writer()` | `FileSink` | Get incremental writer |

### CRITICAL: exists() is async

```typescript
// WRONG - always truthy (returns Promise object)
if (Bun.file("x").exists()) { }

// CORRECT
if (await Bun.file("x").exists()) { }
```

### Bun.write() - Write to Disk

```typescript
await Bun.write(destination, data): Promise<number>
```

**Destinations:** string path, `file://` URL, or BunFile

**Data types:** string, Blob, BunFile, ArrayBuffer, TypedArray, Response

```typescript
// Write string
await Bun.write("output.txt", "hello world");

// Copy file
await Bun.write("dest.txt", Bun.file("src.txt"));

// Write bytes
await Bun.write("out.bin", new Uint8Array([1, 2, 3]));

// Save HTTP response
await Bun.write("page.html", await fetch("https://example.com"));

// Write JSON with formatting
await Bun.write("data.json", JSON.stringify(obj, null, 2));
```

### FileSink - Incremental Writing

```typescript
const writer = Bun.file("output.txt").writer();

writer.write("chunk 1\n");
writer.write("chunk 2\n");
writer.flush();  // flush buffer to disk
writer.end();    // flush and close

// Configure buffer size
const writer = file.writer({ highWaterMark: 1024 * 1024 });
```

### Standard I/O

```typescript
Bun.stdin   // readable
Bun.stdout  // writable
Bun.stderr  // writable
```

### Directory Operations (Use Node.js)

```typescript
import { readdir, mkdir, rm } from "node:fs/promises";

const files = await readdir("./src");
const all = await readdir("./", { recursive: true });
await mkdir("path/to/dir", { recursive: true });
await rm(tempDir, { recursive: true });
```

---

## Bun.Archive

Native API for working with tar archives.

### Constructor

```typescript
new Bun.Archive(data: ArchiveInput, options?: ArchiveOptions)
```

**ArchiveInput:**
- `Record<string, string | Blob | ArrayBufferView>` - files to archive
- `Blob | ArrayBufferView | ArrayBufferLike` - existing archive data

**ArchiveOptions:**
```typescript
{
  compress?: "gzip";  // Enable gzip compression
  level?: number;     // Compression level 1-12 (default: 6)
}
```

### Creating Archives

```typescript
// Create from file map
const archive = new Bun.Archive({
  "hello.txt": "Hello, World!",
  "data.json": JSON.stringify({ foo: "bar" }),
  "nested/file.txt": "Nested content",
});

// Write uncompressed
await Bun.write("output.tar", archive);

// Write compressed
const gzipped = new Bun.Archive(files, { compress: "gzip" });
await Bun.write("output.tar.gz", gzipped);
```

### Extracting Archives

```typescript
const tarball = await Bun.file("package.tar.gz").bytes();
const archive = new Bun.Archive(tarball);

// Extract all
const count = await archive.extract("./output");

// Extract with glob filter
await archive.extract("./output", { glob: "**/*.ts" });

// Multiple patterns
await archive.extract("./output", { glob: ["src/**", "lib/**"] });

// Exclude patterns
await archive.extract("./output", { glob: ["**", "!node_modules/**"] });
```

### Reading Archive Contents

```typescript
const archive = new Bun.Archive(await Bun.file("archive.tar.gz").bytes());
const files = await archive.files();

for (const [path, file] of files) {
  console.log(`${path}: ${file.size} bytes`);
  const content = await file.text();
}

// Filter by glob
const tsFiles = await archive.files("**/*.ts");
```

### Download and Extract Pattern

```typescript
// Common pattern for downloading releases
const url = `https://github.com/org/repo/releases/download/v${version}/app.tar.gz`;
const response = await fetch(url);
if (!response.ok) throw new Error(`Download failed: ${response.status}`);

const tarball = await response.bytes();
const archive = new Bun.Archive(tarball);
await archive.extract("./output");
```

---

## Utilities

### Bun.which()

Find binary in PATH:

```typescript
const path = Bun.which("git");  // "/usr/bin/git" or null
if (!path) {
  console.error("git not found");
}
```

### Bun.argv

CLI arguments:

```typescript
// bun script.ts arg1 arg2
console.log(Bun.argv);
// ["/path/to/bun", "/path/to/script.ts", "arg1", "arg2"]

const args = Bun.argv.slice(2);  // ["arg1", "arg2"]
```

### Bun.sleep()

Async sleep:

```typescript
await Bun.sleep(1000);  // 1 second
await Bun.sleep(100);   // 100ms
```

### import.meta

```typescript
import.meta.main  // true if this file is the entry point
import.meta.dir   // directory containing this file
import.meta.file  // filename of this file
import.meta.path  // full path to this file
```

### process

```typescript
process.exit(0);           // exit with success
process.exit(1);           // exit with error
process.env.HOME           // environment variables
process.cwd()              // current working directory
process.stdout.write("x"); // write without newline
```
