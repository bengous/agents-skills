# Terminology

## Module

A unit with a public interface and private implementation. It can be a function, object, package, directory, service, hook, command, or workflow boundary. The useful question is: what does a caller need to know to use it safely?

## Interface

The public contract callers depend on: functions, types, commands, events, routes, config keys, file formats, and documented behavior. A good interface is smaller than the implementation it hides.

## Implementation

The private logic behind the interface: sequencing, validation, persistence, retries, formatting, adapters, caches, and intermediate data shapes. Deepening moves repeated caller knowledge here.

## Deep Module

A module with a small stable interface that hides substantial implementation. It increases leverage because many callers can rely on one contract instead of recreating the same choreography.

## Shallow Module

A module whose interface is nearly as complex as its implementation. Shallow modules often multiply files and tests without reducing caller knowledge.

## Locality / Colocation

Related decisions live near each other: behavior, schema, validation, tests, docs, fixtures, and adapters for one concept. Colocation is not "put everything in one file"; it is reducing the distance required to understand one change.

## Leverage

How much caller and agent work one interface removes. High leverage looks like one boundary replacing many repeated call sequences, validation branches, or test setups.

## Seam

A boundary where two parts meet. A clear seam exposes stable behavior, owns translation, and prevents callers from depending on internal order, state shape, or transport details.

## Adapter

Code that translates between a module-owned interface and an external shape: filesystem, HTTP, database, CLI, vendor SDK, generated file, or another process. Adapters keep domain logic testable without pretending I/O does not exist.

## AI Entropy

The amount of repo context an AI agent must load to make a safe change. Reduce it by giving concepts obvious owners, stable interfaces, narrow tests, and colocated evidence.

## Candidate Cluster

A set of files, callers, tests, and docs that appear to co-own one concept but lack a sufficiently deep boundary. A candidate is worth exploring only if the proposed boundary could hide meaningful complexity.

## Dependency Categories

- `in-process`: pure computation or local memory. Usually easiest to deepen.
- `local-substitutable`: local I/O with a test stand-in such as temp files, in-memory stores, SQLite, or fake clocks.
- `remote-owned`: an owned network/process boundary. Use ports and adapters.
- `true-external`: third-party systems. Mock or fake at the owned boundary.
