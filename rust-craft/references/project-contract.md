# Project Contract

## Required Decisions

- Inspect every relevant manifest, including workspace inheritance. Record crate kind, edition, `rust-version`, feature graph, resolver, targets, profiles, lint inheritance, and dependency source.
- Inspect `rust-toolchain*`, CI, task runners, release/package configuration, and target-specific dependencies before choosing commands or syntax.
- Distinguish library, binary, proc macro, build script, test/support crate, `no_std`, WASM, embedded, and FFI constraints. They imply different public and runtime contracts.
- Treat the declared MSRV as a compatibility contract: project syntax, standard-library APIs, Cargo manifest syntax, and lint policy must fit it. Verify how the selected Cargo resolver handles dependencies whose `rust-version` exceeds it.
- Treat Cargo features as additive within each resolver unification domain unless the project explicitly defines another policy. Resolver v2 and later separate some target, build, proc-macro, and development contexts; within one domain, `default-features = false` on one edge cannot disable defaults enabled elsewhere.
- Treat `cfg` paths as separate build surfaces. A host build does not validate target-only code.

## Feature, Version, and Target Choices

- Connect an optional dependency explicitly to its feature, commonly with `<feature-name> = ["dep:<dependency-name>"]` when the MSRV supports that syntax; for example, `json = ["dep:serde_json"]`.
- Use mutually exclusive features only when the domain truly requires them and the project owns the resulting combination checks. Prefer additive capabilities.
- Raise `rust-version`, change edition, or adopt newer manifest syntax only as an explicit compatibility decision.
- For virtual workspaces, verify the resolver explicitly. Resolver behavior and MSRV-aware dependency selection vary by Cargo version.
- Use `rustc --print cfg --target <triple>` when the target's configuration, not intuition, is the question.

## Validation Ladder

1. Run the repository's formatter, checker, tests, and lints for the smallest affected package and target.
2. Exercise default, no-default, selected, or all-feature builds only where those combinations are promised. `--all-features` is not a substitute for meaningful combinations.
3. Check the declared MSRV when the change uses version-sensitive language, library, Cargo, or dependency behavior.
4. Build or test each affected target-specific path on its target or an established equivalent.
5. Check docs, packaging, unsafe code, concurrency, or performance with the specialized evidence in the other references.

Common tools, when the project contract calls for them, include `cargo metadata`, `cargo tree -e features`, `cargo check --all-targets`, `cargo test`, `cargo clippy`, `cargo doc`, and `cargo package`. Preserve existing `--locked`, workspace, target, and lint policies rather than inventing them.

## Failure Modes

- Copying commands such as `--workspace --all-targets --all-features --locked -- -D warnings` into every project.
- Using an API from current stable Rust without checking a lower declared MSRV.
- Assuming `cargo test` covers disabled features, target-specific modules, examples, or packaged contents.
- Silently changing `Cargo.lock`, dependency features, resolver, edition, or supported toolchain as part of an unrelated fix.
- Treating edition migration as formatting or assuming it changes the runtime automatically.

## Official Sources

- [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html)
- [Cargo `rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html)
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [Rust conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [Cargo target-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [Rust editions](https://doc.rust-lang.org/edition-guide/editions/index.html)
