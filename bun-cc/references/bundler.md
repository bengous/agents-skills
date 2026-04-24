# Bundler and Executables

Bun's bundler handles TypeScript, JSX/TSX, CSS, assets, and Bun-native runtime
targets. Use it directly for simple builds; keep framework build systems when a
project already has one.

## Basic Build

```typescript
const result = await Bun.build({
  entrypoints: ["./src/index.ts"],
  outdir: "./dist",
  target: "bun",
});

if (!result.success) {
  throw new AggregateError(result.logs, "Build failed");
}
```

## Common Options

```typescript
const result = await Bun.build({
  entrypoints: ["./src/index.ts", "./src/worker.ts"],
  outdir: "./dist",
  target: "bun",            // "browser" | "bun" | "node"
  format: "esm",            // "esm" | "cjs" | "iife"
  splitting: true,           // ESM code splitting
  minify: true,
  sourcemap: "linked",       // "none" | "linked" | "inline" | "external"

  external: ["react"],
  packages: "external",      // mark all package imports external

  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },

  env: "inline",             // or prefix such as "PUBLIC_*"
  loader: { ".txt": "text", ".png": "file" },
  naming: {
    entry: "[dir]/[name]-[hash].[ext]",
    chunk: "chunks/[name]-[hash].[ext]",
    asset: "assets/[name]-[hash].[ext]",
  },

  drop: ["console", "debugger"],
  metafile: true,
});

for (const output of result.outputs) {
  console.log(output.kind, output.path);
}
```

Notes:

- If an entrypoint has `#!/usr/bin/env bun`, Bun defaults to `target: "bun"`.
- Bundles with `target: "bun"` include a `// @bun` pragma so Bun does not
  retranspile them at runtime.
- `packages: "external"` is useful for server builds that should keep runtime
  dependencies external.

## Standalone Executables

Prefer the current object form for API builds:

```typescript
const result = await Bun.build({
  entrypoints: ["./src/cli.ts"],
  compile: {
    target: "bun-linux-x64",
    outfile: "./dist/mycli",
    execArgv: ["--smol"],
  },
  minify: true,
  bytecode: true,
});

if (!result.success) {
  throw new AggregateError(result.logs, "Executable build failed");
}
```

CLI equivalent:

```bash
bun build ./src/cli.ts --compile --outfile mycli
bun build ./src/cli.ts --compile --target=bun-linux-x64 --outfile mycli-linux
```

Supported target families:

| Target | OS | Arch | Libc |
|---|---|---|---|
| `bun-linux-x64` | Linux | x64 | glibc |
| `bun-linux-arm64` | Linux | arm64 | glibc |
| `bun-linux-x64-musl` | Linux | x64 | musl |
| `bun-linux-arm64-musl` | Linux | arm64 | musl |
| `bun-darwin-x64` | macOS | x64 | system |
| `bun-darwin-arm64` | macOS | arm64 | system |
| `bun-windows-x64` | Windows | x64 | system |
| `bun-windows-arm64` | Windows | arm64 | system |

For x64 targets, append `-baseline` for older CPUs or `-modern` for newer CPUs.
Use `-baseline` when users report `Illegal instruction` on older Linux/Windows
machines.

## Guardrails

- Compile CLIs from a single entrypoint; keep dynamic plugin loading external or
  test it in the compiled binary.
- Test the generated executable, not only the TypeScript source.
- Do not assume cross-compiled binaries behave identically across OS path,
  permission, and shell semantics.
- For Windows executables, Bun adds `.exe` when no extension is provided.

## Official Docs

- `https://bun.sh/docs/bundler`
- `https://bun.sh/docs/bundler/executables`
