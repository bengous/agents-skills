#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync, mkdirSync, readdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import path from "node:path";

const rootDir = process.env.STATE_MACHINE_ROOT;
const repoRoot = process.env.STATE_MACHINE_REPO_ROOT;
const validationDir = process.env.STATE_MACHINE_VALIDATION_DIR;
const workRoot = process.env.STATE_MACHINE_WORK_ROOT;
const nodeModules = process.env.STATE_MACHINE_NODE_MODULES;

if (!rootDir || !repoRoot || !validationDir || !workRoot || !nodeModules) {
  throw new Error("missing validation environment; run scripts/validate-references.sh");
}

const requireFromScratch = createRequire(path.join(nodeModules, "package.json"));
const MarkdownIt = requireFromScratch("markdown-it");
const markdown = new MarkdownIt({ html: true });

const manifestPath = path.join(validationDir, "snippets.json");
const snippetsManifest = JSON.parse(readFileSync(manifestPath, "utf8"));

const jsWork = path.join(workRoot, "js");
const generatedRoot = path.join(workRoot, "generated");
const tsDir = path.join(generatedRoot, "ts");
const jsDir = path.join(generatedRoot, "js");
const svelteDir = path.join(generatedRoot, "svelte");
const cDir = path.join(generatedRoot, "c");
const javaDir = path.join(generatedRoot, "java");
const rustDir = path.join(generatedRoot, "rust");
const mermaidDir = path.join(generatedRoot, "mermaid");
const shellDir = path.join(generatedRoot, "shell");
const tomlDir = path.join(generatedRoot, "toml");

for (const dir of [tsDir, jsDir, svelteDir, cDir, javaDir, rustDir, mermaidDir, shellDir, tomlDir]) {
  mkdirSync(dir, { recursive: true });
}

function rel(file) {
  return path.relative(rootDir, file).split(path.sep).join("/");
}

function sha256(text) {
  return createHash("sha256").update(text).digest("hex");
}

function run(label, command, args, options = {}) {
  console.log(`\n==> ${label}`);
  execFileSync(command, args, {
    cwd: options.cwd ?? rootDir,
    env: { ...process.env, ...(options.env ?? {}) },
    stdio: "inherit",
  });
}

function localBin(name) {
  const binName = process.platform === "win32" ? `${name}.cmd` : name;
  return path.join(nodeModules, ".bin", binName);
}

function listMarkdownFiles() {
  const files = [path.join(rootDir, "SKILL.md")];
  const referencesDir = path.join(rootDir, "references");
  const walk = (dir) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.isFile() && entry.name.endsWith(".md")) {
        files.push(full);
      }
    }
  };
  walk(referencesDir);
  return files.sort();
}

function readFences() {
  const fences = [];
  for (const file of listMarkdownFiles()) {
    const source = readFileSync(file, "utf8");
    const tokens = markdown.parse(source, {});
    let index = 0;
    for (const token of tokens) {
      if (token.type !== "fence") continue;
      index += 1;
      const info = token.info.trim();
      const lang = info.split(/\s+/)[0] || "";
      const startLine = (token.map?.[0] ?? 0) + 1;
      const endLine = token.map?.[1] ?? startLine;
      fences.push({
        file: rel(file),
        index,
        lang,
        info,
        startLine,
        endLine,
        content: token.content.replace(/\n$/, ""),
      });
    }
  }
  return fences;
}

const fences = readFences();
const fenceByKey = new Map(fences.map((fence) => [`${fence.file}#${fence.index}`, fence]));
const manifestByKey = new Map(snippetsManifest.snippets.map((item) => [`${item.file}#${item.index}`, item]));

function validateCoverage() {
  const errors = [];
  for (const fence of fences) {
    const key = `${fence.file}#${fence.index}`;
    const item = manifestByKey.get(key);
    if (!item) {
      errors.push(`missing manifest entry for ${fence.file} fence ${fence.index} (${fence.lang || "no-lang"})`);
      continue;
    }
    if (item.lang !== fence.lang) {
      errors.push(`${key} language changed: manifest=${item.lang} actual=${fence.lang}`);
    }
    const hash = sha256(fence.content);
    if (item.sha256 !== hash) {
      errors.push(`${key} content hash changed at line ${fence.startLine}; update fixture/manifest intentionally`);
    }
    if ((item.mode === "text" || item.mode.endsWith("-sketch")) && !item.reason) {
      errors.push(`${key} is non-executable but has no reason`);
    }
  }
  for (const item of snippetsManifest.snippets) {
    const key = `${item.file}#${item.index}`;
    if (!fenceByKey.has(key)) {
      errors.push(`stale manifest entry for ${key}`);
    }
  }
  if (errors.length > 0) {
    throw new Error(`snippet manifest coverage failed:\n${errors.map((e) => `- ${e}`).join("\n")}`);
  }
  console.log(`validated snippet manifest coverage for ${fences.length} fences`);
}

function sanitizeFileName(item, ext) {
  const base = item.file
    .replace(/^references\//, "")
    .replace(/\.md$/, "")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return `${String(item.index).padStart(2, "0")}_${base}.${ext}`;
}

function getFence(item) {
  const fence = fenceByKey.get(`${item.file}#${item.index}`);
  if (!fence) throw new Error(`missing parsed fence for ${item.file}#${item.index}`);
  return fence;
}

function hasDeclaration(body, name) {
  return new RegExp(`\\b(type|interface|class|enum)\\s+${name}\\b`).test(body);
}

function tsPrelude(body) {
  const lines = [];
  const typeIfMissing = (name, source) => {
    if (!hasDeclaration(body, name)) lines.push(source);
  };
  typeIfMissing("T", "type T = string;");
  typeIfMissing("Item", "type Item = { id: string; name?: string };");
  typeIfMissing("User", "type User = { id: string; name: string };");
  typeIfMissing("UserEdits", "type UserEdits = Partial<User>;");
  typeIfMissing("Token", "type Token = string;");
  typeIfMissing("Credentials", "type Credentials = { username: string; password: string };");
  typeIfMissing("FormValues", "type FormValues = Record<string, string>;");
  typeIfMissing("ValidationErrors", "type ValidationErrors = Record<string, string>;");
  typeIfMissing("SubmitResult", "type SubmitResult = { id: string };");
  typeIfMissing("SubmitError", "type SubmitError = Error;");
  typeIfMissing(
    "State",
    'type State =\n  | { status: "idle" }\n  | { status: "loading" }\n  | { status: "success"; data: string }\n  | { status: "error"; error: Error };',
  );
  typeIfMissing(
    "MachineEvent",
    'type MachineEvent =\n  | { type: "FETCH" }\n  | { type: "RESPONSE_OK"; data: string }\n  | { type: "RESPONSE_ERROR"; error: Error }\n  | { type: "RETRY" }\n  | { type: "REFETCH" };',
  );
  if (body.includes("reduce(") && !/\bfunction\s+reduce\b/.test(body)) {
    lines.push("declare function reduce(state: State, event: MachineEvent): State;");
  }
  if (body.includes("useState")) {
    lines.push("declare function useState<S>(initial: S): [S, (next: S | ((prev: S) => S)) => void];");
  }
  if (body.includes("useMemo(")) {
    lines.push("declare function useMemo<TValue>(factory: () => TValue, deps: readonly unknown[]): TValue;");
  }
  if (body.includes("useUserQuery(")) {
    lines.push(
      "declare function useUserQuery(): { data: User | undefined; status: string; isFetching: boolean; error: Error | null };",
    );
  }
  return lines.length > 0 ? `${lines.join("\n")}\n\n` : "";
}

function splitLeadingImports(body) {
  const lines = body.split("\n");
  const imports = [];
  const rest = [];
  let inLeadingImports = true;
  for (const line of lines) {
    if (inLeadingImports && (line.startsWith("import ") || line.trim() === "")) {
      imports.push(line);
      continue;
    }
    inLeadingImports = false;
    rest.push(line);
  }
  return { imports: imports.join("\n").trim(), rest: rest.join("\n") };
}

function writeTypeScriptFixtures() {
  const tsNodeModules = path.join(tsDir, "node_modules");
  if (!existsSync(tsNodeModules)) {
    symlinkSync(nodeModules, tsNodeModules, "dir");
  }
  const tsItems = snippetsManifest.snippets.filter((item) => item.mode === "ts-typecheck");
  for (const item of tsItems) {
    const fence = getFence(item);
    const { imports, rest } = splitLeadingImports(fence.content);
    const content = [
      `// Source: ${item.file} fence ${item.index}, line ${fence.startLine}`,
      imports,
      tsPrelude(rest),
      rest,
      "export {};",
      "",
    ]
      .filter(Boolean)
      .join("\n");
    writeFileSync(path.join(tsDir, sanitizeFileName(item, "ts")), content);
  }

  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "tsx-typecheck")) {
    const fence = getFence(item);
    const content = `// Source: ${item.file} fence ${item.index}, line ${fence.startLine}
import * as React from "react";

type GeoState =
  | { status: "idle" }
  | { status: "pending"; position: GeolocationPosition | null }
  | { status: "resolved"; position: GeolocationPosition }
  | { status: "rejected"; error: Error };

declare function useGeoPosition(): GeoState;
declare function format(position: GeolocationPosition): string;

${fence.content}

void React;
export {};
`;
    writeFileSync(path.join(tsDir, sanitizeFileName(item, "tsx")), content);
  }

  writeFileSync(
    path.join(tsDir, "machine.ts"),
    `import { createMachine } from "xstate";

export const machine = createMachine({
  initial: "idle",
  states: {
    idle: {},
  },
});
`,
  );

  writeFileSync(
    path.join(tsDir, "tsconfig.json"),
    JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          module: "ESNext",
          moduleResolution: "Bundler",
          lib: ["ES2022", "DOM", "DOM.Iterable"],
          jsx: "react-jsx",
          strict: true,
          noUncheckedIndexedAccess: true,
          exactOptionalPropertyTypes: true,
          noImplicitOverride: true,
          noPropertyAccessFromIndexSignature: true,
          useUnknownInCatchVariables: true,
          verbatimModuleSyntax: true,
          isolatedModules: true,
          moduleDetection: "force",
          skipLibCheck: true,
          noEmit: true,
        },
        include: ["**/*.ts", "**/*.tsx"],
      },
      null,
      2,
    ),
  );
}

function writeJavaScriptFixtures() {
  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "js-syntax")) {
    const fence = getFence(item);
    const content = `// Source: ${item.file} fence ${item.index}, line ${fence.startLine}
const React = {
  useReducer(reducer, initialState) {
    void reducer;
    return [initialState, () => undefined];
  },
};

${fence.content}
`;
    const file = path.join(jsDir, sanitizeFileName(item, "js"));
    writeFileSync(file, content);
    run(`node --check ${path.relative(workRoot, file)}`, process.execPath, ["--check", file]);
  }
}

function writeSvelteFixtures() {
  const svelteNodeModules = path.join(svelteDir, "node_modules");
  if (!existsSync(svelteNodeModules)) {
    symlinkSync(nodeModules, svelteNodeModules, "dir");
  }
  const srcDir = path.join(svelteDir, "src");
  mkdirSync(srcDir, { recursive: true });
  writeFileSync(path.join(svelteDir, "package.json"), JSON.stringify({ type: "module" }, null, 2));
  writeFileSync(
    path.join(svelteDir, "tsconfig.json"),
    JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          module: "ESNext",
          moduleResolution: "Bundler",
          lib: ["ES2022", "DOM", "DOM.Iterable"],
          strict: true,
          noUncheckedIndexedAccess: true,
          exactOptionalPropertyTypes: true,
          isolatedModules: true,
          skipLibCheck: true,
          allowImportingTsExtensions: true,
          noEmit: true,
          types: ["svelte"],
        },
        include: ["src/**/*"],
      },
      null,
      2,
    ),
  );
  writeFileSync(
    path.join(svelteDir, "svelte.config.js"),
    `export default {
  compilerOptions: {
    runes: true
  }
};
`,
  );

  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "svelte-module")) {
    const fence = getFence(item);
    const fileName = item.fixture === "cart" ? "cart.svelte.ts" : "fsm.svelte.ts";
    const prelude = item.fixture === "cart" ? "type CartItem = { id: string };\n\n" : "";
    writeFileSync(path.join(srcDir, fileName), `${prelude}${fence.content}\n`);
  }

  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "svelte-component")) {
    const fence = getFence(item);
    writeFileSync(path.join(srcDir, `${sanitizeFileName(item, "svelte")}`), `${fence.content}\n`);
  }

  const svelteApi = snippetsManifest.snippets.find(
    (entry) => entry.file === "references/implementations/svelte5-runes.md" && entry.index === 4,
  );
  if (svelteApi) {
    writeFileSync(path.join(srcDir, "tiny-svelte-fsm.ts"), `${getFence(svelteApi).content}\nexport {};\n`);
  }

  writeFileSync(
    path.join(srcDir, "toggle.machine.ts"),
    `import { createMachine } from "xstate";

export const toggle = createMachine({
  initial: "inactive",
  states: {
    inactive: { on: { TOGGLE: "active" } },
    active: { on: { TOGGLE: "inactive" } },
  },
});
`,
  );
}

function writeCFixtures() {
  const cItems = snippetsManifest.snippets.filter((entry) => entry.mode === "c-compile");
  const flags = [
    "-std=c17",
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Werror",
    "-Wconversion",
    "-Wsign-conversion",
    "-Wshadow",
    "-Wswitch-enum",
    "-Wstrict-prototypes",
    "-Wold-style-definition",
    "-Wundef",
    "-Wformat=2",
    "-Wnull-dereference",
  ];
  const compilers = ["gcc", "clang"];
  for (const item of cItems) {
    const fence = getFence(item);
    const file = path.join(cDir, sanitizeFileName(item, "c"));
    writeFileSync(file, `${fence.content}\n`);
    for (const compiler of compilers) {
      run(`${compiler} strict C check for ${item.file} fence ${item.index}`, compiler, [
        ...flags,
        "-c",
        file,
        "-o",
        path.join(cDir, `${path.basename(file)}.${compiler}.o`),
      ]);
    }
  }
}

function writeJavaFixtures() {
  const classesDir = path.join(javaDir, "classes");
  mkdirSync(classesDir, { recursive: true });
  const compileItems = snippetsManifest.snippets.filter((entry) => entry.mode === "java-compile");
  for (const item of compileItems) {
    const fence = getFence(item);
    const fileName = item.fixture === "order-state" ? "OrderState.java" : "StatePattern.java";
    const file = path.join(javaDir, fileName);
    writeFileSync(file, `${fence.content}\n`);
    run(`javac strict check for ${item.file} fence ${item.index}`, "javac", [
      "--release",
      "17",
      "-encoding",
      "UTF-8",
      "-Xlint:all",
      "-Werror",
      "-proc:none",
      "-implicit:none",
      "-d",
      classesDir,
      file,
    ]);
  }
}

function writeRustFixture() {
  mkdirSync(path.join(rustDir, "src"), { recursive: true });
  for (const name of ["Cargo.toml", "Cargo.lock"]) {
    writeFileSync(
      path.join(rustDir, name),
      readFileSync(path.join(validationDir, "toolchains", "cargo", name), "utf8"),
    );
  }
  writeFileSync(
    path.join(rustDir, "Cargo.toml"),
    `${readFileSync(path.join(rustDir, "Cargo.toml"), "utf8")}
[lib]
path = "src/lib.rs"
`,
  );
  writeFileSync(
    path.join(rustDir, "src", "lib.rs"),
    `#![deny(warnings)]
#![deny(unsafe_code)]

/// Validates that rustdoc is wired into the snippet fixture.
///
/// \`\`\`rust
/// assert_eq!("state-machine", "state-machine");
/// \`\`\`
pub fn doctest_anchor() {}
`,
  );
  writeFileSync(
    path.join(rustDir, "src", "main.rs"),
    `#![deny(warnings)]
#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod enum_match {
    enum State {
        Idle,
        Loading { attempt: u8 },
        Success { data: String },
        Error { error: String, attempt: u8 },
        GiveUp { last_error: String },
    }

    enum Event {
        Fetch,
        ResponseOk(String),
        ResponseError(String),
        Retry,
        Refetch,
    }

    const MAX_ATTEMPTS: u8 = 3;

    fn reduce(state: State, event: Event) -> State {
        match state {
            State::Idle => match event {
                Event::Fetch => State::Loading { attempt: 1 },
                Event::ResponseOk(_) | Event::ResponseError(_) | Event::Retry | Event::Refetch => {
                    State::Idle
                }
            },
            State::Loading { attempt } => match event {
                Event::ResponseOk(data) => State::Success { data },
                Event::ResponseError(error) => State::Error { error, attempt },
                Event::Fetch | Event::Retry | Event::Refetch => State::Loading { attempt },
            },
            State::Success { data } => match event {
                Event::Refetch => State::Loading { attempt: 1 },
                Event::Fetch | Event::ResponseOk(_) | Event::ResponseError(_) | Event::Retry => {
                    State::Success { data }
                }
            },
            State::Error { error, attempt } => match event {
                Event::Retry if attempt < MAX_ATTEMPTS => State::Loading {
                    attempt: attempt + 1,
                },
                Event::Retry => State::GiveUp { last_error: error },
                Event::Fetch | Event::ResponseOk(_) | Event::ResponseError(_) | Event::Refetch => {
                    State::Error { error, attempt }
                }
            },
            State::GiveUp { last_error } => match event {
                Event::Retry => State::Loading { attempt: 1 },
                Event::Fetch | Event::ResponseOk(_) | Event::ResponseError(_) | Event::Refetch => {
                    State::GiveUp { last_error }
                }
            },
        }
    }

    pub fn exercise() {
        let state = reduce(State::Idle, Event::Fetch);
        let state = reduce(state, Event::ResponseError(String::from("timeout")));
        let state = reduce(state, Event::Retry);
        let state = reduce(state, Event::ResponseOk(String::from("ok")));
        let state = reduce(state, Event::Refetch);
        let state = reduce(state, Event::ResponseError(String::from("still down")));
        let state = reduce(state, Event::Retry);
        let state = reduce(state, Event::ResponseError(String::from("still down")));
        let state = reduce(state, Event::Retry);
        let state = reduce(state, Event::ResponseError(String::from("still down")));
        let state = reduce(state, Event::Retry);

        match state {
            State::GiveUp { last_error } => assert_eq!(last_error, "still down"),
            State::Success { data } => assert_eq!(data, "ok"),
            State::Loading { attempt } => assert!(attempt > 0),
            State::Error { error, attempt } => {
                assert!(!error.is_empty());
                assert!(attempt > 0);
            }
            State::Idle => {}
        }
    }
}

mod typestate {
    pub struct Idle;
    pub struct Loading {
        attempt: u8,
    }
    pub struct Success {
        pub data: String,
    }
    pub struct Error {
        pub error: String,
        attempt: u8,
    }

    impl Idle {
        pub fn fetch(self) -> Loading {
            Loading { attempt: 1 }
        }
    }

    impl Loading {
        pub fn response_ok(self, data: String) -> Success {
            Success { data }
        }

        pub fn response_error(self, error: String) -> Error {
            Error {
                error,
                attempt: self.attempt,
            }
        }
    }

    impl Error {
        pub fn retry(self) -> Result<Loading, GaveUp> {
            if self.attempt < 3 {
                Ok(Loading {
                    attempt: self.attempt + 1,
                })
            } else {
                Err(GaveUp {
                    last_error: self.error,
                })
            }
        }
    }

    pub struct GaveUp {
        pub last_error: String,
    }

    pub fn exercise() {
        let loading = Idle.fetch();
        let success = loading.response_ok(String::from("hello"));
        assert_eq!(success.data, "hello");

        let retryable = Loading { attempt: 1 }.response_error(String::from("temporary"));
        assert!(retryable.retry().is_ok());

        let gave_up = Error {
            error: String::from("permanent"),
            attempt: 3,
        }
        .retry();
        match gave_up {
            Ok(_) => panic!("expected give up"),
            Err(error) => assert_eq!(error.last_error, "permanent"),
        }
    }
}

mod statig_hsm {
    use statig::prelude::*;

    #[derive(Default)]
    struct Fetch;

    pub enum Event {
        Start,
        ResponseOk(String),
        ResponseError(String),
        Retry,
    }

    #[state_machine(initial = "State::idle()")]
    impl Fetch {
        #[state]
        fn idle(event: &Event) -> Outcome<State> {
            match event {
                Event::Start => Transition(State::loading(1)),
                Event::ResponseOk(_) | Event::ResponseError(_) | Event::Retry => Super,
            }
        }

        #[state]
        fn loading(attempt: &u8, event: &Event) -> Outcome<State> {
            match event {
                Event::ResponseOk(data) => Transition(State::success(data.clone())),
                Event::ResponseError(err) => Transition(State::error(err.clone(), *attempt)),
                Event::Start | Event::Retry => Super,
            }
        }

        #[state]
        fn success(_data: &String, event: &Event) -> Outcome<State> {
            match event {
                Event::Start => Transition(State::loading(1)),
                Event::ResponseOk(_) | Event::ResponseError(_) | Event::Retry => Super,
            }
        }

        #[state]
        #[allow(clippy::ptr_arg)]
        fn error(error: &String, attempt: &u8, event: &Event) -> Outcome<State> {
            match event {
                Event::Retry if *attempt < 3 => Transition(State::loading(*attempt + 1)),
                Event::Retry => Transition(State::give_up(error.clone())),
                Event::Start | Event::ResponseOk(_) | Event::ResponseError(_) => Super,
            }
        }

        #[state]
        fn give_up(_last_error: &String, event: &Event) -> Outcome<State> {
            match event {
                Event::Retry => Transition(State::loading(1)),
                Event::Start | Event::ResponseOk(_) | Event::ResponseError(_) => Super,
            }
        }
    }

    pub fn exercise() {
        let mut sm = Fetch.state_machine();
        sm.handle(&Event::Start);
        sm.handle(&Event::ResponseError(String::from("temporary")));
        sm.handle(&Event::Retry);
        sm.handle(&Event::ResponseOk(String::from("ok")));
    }
}

fn main() {
    enum_match::exercise();
    typestate::exercise();
    statig_hsm::exercise();
}
`,
  );
}

function validateMermaid() {
  const items = snippetsManifest.snippets.filter((entry) => entry.mode === "mermaid-render");
  const puppeteerConfig = path.join(mermaidDir, "puppeteer.json");
  writeFileSync(puppeteerConfig, JSON.stringify({ args: ["--no-sandbox", "--disable-setuid-sandbox"] }, null, 2));
  const chromium = process.env.PUPPETEER_EXECUTABLE_PATH || "/usr/bin/chromium";
  for (const item of items) {
    const fence = getFence(item);
    const input = path.join(mermaidDir, sanitizeFileName(item, "mmd"));
    const output = path.join(mermaidDir, `${path.basename(input)}.svg`);
    writeFileSync(input, `${fence.content}\n`);
    run(`mermaid render ${item.file} fence ${item.index}`, localBin("mmdc"), [
      "-i",
      input,
      "-o",
      output,
      "-p",
      puppeteerConfig,
      "-q",
    ], {
      env: existsSync(chromium) ? { PUPPETEER_EXECUTABLE_PATH: chromium } : {},
    });
  }
}

function validateShell() {
  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "shellcheck")) {
    const fence = getFence(item);
    const file = path.join(shellDir, sanitizeFileName(item, "sh"));
    writeFileSync(file, `#!/usr/bin/env bash\nset -euo pipefail\n${fence.content}\n`);
    run(`shellcheck ${item.file} fence ${item.index}`, "shellcheck", [
      "--shell=bash",
      "--severity=style",
      "--format=gcc",
      file,
    ]);
  }
}

function validateToml() {
  for (const item of snippetsManifest.snippets.filter((entry) => entry.mode === "toml-check")) {
    const fence = getFence(item);
    const file = path.join(tomlDir, sanitizeFileName(item, "toml"));
    writeFileSync(file, `${fence.content}\n`);
    run(`taplo check ${item.file} fence ${item.index}`, localBin("taplo"), ["check", "--no-auto-config", file], {
      env: { RUST_LOG: "error" },
    });
  }
}

function validateMarkdown() {
  run("markdownlint-cli2", localBin("markdownlint-cli2"), [
    "--config",
    path.join(validationDir, "toolchains", "npm", "markdownlint-cli2.jsonc"),
    "SKILL.md",
    "references/**/*.md",
  ]);

  run("frontmatter validator", "cargo", [
    "run",
    "--manifest-path",
    path.join(repoRoot, "Cargo.toml"),
    "-p",
    "skills-tools",
    "--",
    "validate",
    "frontmatter",
    path.join(rootDir, "SKILL.md"),
  ], {
    env: {
      CARGO_HOME: path.join(workRoot, "cargo-home"),
      CARGO_TARGET_DIR: path.join(workRoot, "repo-target"),
    },
  });

  const linkConfig = path.join(validationDir, "toolchains", "npm", "markdown-link-check.json");
  for (const file of listMarkdownFiles()) {
    run(`markdown-link-check ${rel(file)}`, process.execPath, [
      path.join(nodeModules, "markdown-link-check", "markdown-link-check"),
      "-c",
      linkConfig,
      file,
    ]);
  }
}

function runTypeScript() {
  writeTypeScriptFixtures();
  run("tsc strict snippets", localBin("tsc"), ["-p", path.join(tsDir, "tsconfig.json"), "--noEmit"]);
}

function runSvelte() {
  writeSvelteFixtures();
  run("svelte-check strict snippets", localBin("svelte-check"), [
    "--tsconfig",
    path.join(svelteDir, "tsconfig.json"),
    "--fail-on-warnings",
    "--output",
    "machine-verbose",
  ], {
    cwd: svelteDir,
  });
}

function runRust() {
  writeRustFixture();
  const env = {
    CARGO_HOME: path.join(workRoot, "cargo-home"),
    CARGO_TARGET_DIR: path.join(workRoot, "rust-target"),
  };
  run("cargo fmt --check rust snippets", "cargo", ["fmt", "--manifest-path", path.join(rustDir, "Cargo.toml"), "--check"], {
    env,
  });
  run("cargo check rust snippets", "cargo", ["check", "--manifest-path", path.join(rustDir, "Cargo.toml"), "--locked", "--all-targets"], {
    env,
  });
  run("cargo clippy rust snippets", "cargo", [
    "clippy",
    "--manifest-path",
    path.join(rustDir, "Cargo.toml"),
    "--locked",
    "--all-targets",
    "--",
    "-D",
    "warnings",
  ], {
    env,
  });
  run("cargo test --doc rust snippets", "cargo", ["test", "--manifest-path", path.join(rustDir, "Cargo.toml"), "--locked", "--doc"], {
    env,
  });
}

validateCoverage();
writeJavaScriptFixtures();
runTypeScript();
runSvelte();
writeCFixtures();
writeJavaFixtures();
runRust();
validateMermaid();
validateShell();
validateToml();
validateMarkdown();

console.log(`\nvalidated ${fences.length} fenced snippets; scratch retained at ${workRoot}`);
