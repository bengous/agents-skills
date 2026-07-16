# Release contract

`wire-core` is a public library. Patch releases must preserve:

- the workspace MSRV;
- existing downstream `Encoder` implementations and `Box<dyn Encoder>` users;
- default and `schema`-only builds;
- a package containing every file used by enabled library code;
- the lockfile and dependency set unless the release task explicitly changes dependencies.

`./release-check.sh` is the native release gate.
