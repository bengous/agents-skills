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

console.log(result.outputs.map((output) => output.path));
```

`Bun.build()` rejects on failure by default. Catch `AggregateError` for normal
failure handling:

```typescript
try {
  await Bun.build({
    entrypoints: ["./src/index.ts"],
    outdir: "./dist",
    target: "bun",
  });
} catch (error) {
  const aggregate = error as AggregateError;
  console.error(aggregate.errors);
  process.exit(1);
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

- JavaScript API builds default to `target: "browser"` unless you set `target`.
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
    autoloadDotenv: false,
    autoloadBunfig: false,
  },
  minify: true,
  bytecode: true,
});

console.log(result.outputs[0]?.path);
```

CLI equivalent:

```bash
bun build ./src/cli.ts --compile --outfile mycli
bun build ./src/cli.ts --compile --target=bun-linux-x64 --outfile mycli-linux
```

Common documented targets:

| Target | OS | Arch | Libc |
|---|---|---|---|
| `bun-linux-x64` | Linux | x64 | glibc |
| `bun-linux-x64-baseline` | Linux | x64 | glibc |
| `bun-linux-x64-modern` | Linux | x64 | glibc |
| `bun-linux-arm64` | Linux | arm64 | glibc |
| `bun-linux-x64-musl` | Linux | x64 | musl |
| `bun-linux-x64-musl-baseline` | Linux | x64 | musl |
| `bun-linux-x64-musl-modern` | Linux | x64 | musl |
| `bun-linux-arm64-musl` | Linux | arm64 | musl |
| `bun-darwin-x64` | macOS | x64 | system |
| `bun-darwin-x64-baseline` | macOS | x64 | system |
| `bun-darwin-arm64` | macOS | arm64 | system |
| `bun-windows-x64` | Windows | x64 | system |
| `bun-windows-x64-baseline` | Windows | x64 | system |
| `bun-windows-x64-modern` | Windows | x64 | system |
| `bun-windows-arm64` | Windows | arm64 | system |

Prefer documented `arm64` target strings in build scripts. Bun's types also accept
some `aarch64` aliases. Use `-baseline` when users report `Illegal instruction`
on older x64 machines; `-modern` is most relevant for Linux and Windows x64.

`compile` accepts three JavaScript API forms:

```typescript
compile: true;
compile: "bun-linux-x64";
compile: { target: "bun-linux-x64", outfile: "./mycli" };
```

With `target: "browser"` and HTML entrypoints, `--compile` produces self-contained
HTML rather than a native executable.

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
