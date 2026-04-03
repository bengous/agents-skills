# Bundler & Compiler

Bun's native bundler via `Bun.build()` and standalone executable compilation.

## Basic Bundling

```typescript
const result = await Bun.build({
  entrypoints: ["./src/index.tsx"],
  outdir: "./dist",
});

if (!result.success) {
  console.error("Build failed:", result.logs);
}
```

## Full Configuration

```typescript
await Bun.build({
  entrypoints: ["./src/index.tsx", "./src/worker.ts"],
  outdir: "./dist",
  target: "browser",       // "browser" | "bun" | "node"
  format: "esm",           // "esm" | "cjs" | "iife"
  splitting: true,          // code splitting (ESM only)
  minify: true,             // or { whitespace: true, syntax: true, identifiers: true }
  sourcemap: "linked",      // "none" | "linked" | "inline" | "external"

  // Don't bundle these packages
  external: ["react", "react-dom"],
  // Or mark ALL packages external
  packages: "external",

  // Compile-time constants
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
    VERSION: JSON.stringify("1.0.0"),
  },

  // Inline env vars (or "PUBLIC_*" for prefix matching)
  env: "inline",

  // Custom file loaders
  loader: { ".png": "dataurl", ".txt": "file" },

  // Output naming with tokens: [name], [hash], [dir], [ext]
  naming: {
    entry: "[dir]/[name]-[hash].[ext]",
    chunk: "chunks/[name]-[hash].[ext]",
    asset: "assets/[name]-[hash].[ext]",
  },

  // JSX config
  jsx: { runtime: "automatic", importSource: "react" },

  // Banner/footer
  banner: '"use client";',
  footer: "// Built with Bun",

  // Remove console/debugger
  drop: ["console", "debugger"],

  // Build metadata
  metafile: true,
});
```

## Build Result

```typescript
const result = await Bun.build({ /* ... */ });

result.success;   // boolean
result.logs;      // build messages (errors/warnings)
result.outputs;   // BuildArtifact[]

for (const output of result.outputs) {
  output.path;    // absolute path
  output.kind;    // "entry-point" | "chunk" | "asset" | "sourcemap"
  await output.text();   // file contents as string
}
```

## Compile to Standalone Executable

Bundle + embed the Bun runtime into a single binary:

```typescript
// API
const result = await Bun.build({
  entrypoints: ["./src/cli.ts"],
  compile: true,               // current platform
  // OR
  compile: "bun-linux-x64",   // cross-compile
  // OR
  compile: {
    target: "bun-linux-x64",
    outfile: "./dist/mycli",
    execArgv: ["--smol"],       // reduce memory usage
  },
  minify: true,
  sourcemap: "linked",
  bytecode: true,               // pre-compile to bytecode
});
```

### CLI equivalent

```bash
bun build ./src/cli.ts --compile --outfile mycli
bun build ./src/cli.ts --compile --target=bun-linux-x64 --outfile mycli-linux
```

### Cross-compilation targets

| Target | OS | Arch |
|---|---|---|
| `bun-linux-x64` | Linux | x64 |
| `bun-linux-arm64` | Linux | ARM64 |
| `bun-darwin-x64` | macOS | x64 |
| `bun-darwin-arm64` | macOS | ARM64 (Apple Silicon) |
| `bun-windows-x64` | Windows | x64 |
| `bun-windows-arm64` | Windows | ARM64 |

Variants: append `-baseline` (older CPUs), `-modern` (newer CPUs), or `-musl` (Alpine Linux).

## Official Docs

- [Bundler](https://bun.sh/docs/bundler)
- [Standalone Executables](https://bun.sh/docs/bundler/executables)
