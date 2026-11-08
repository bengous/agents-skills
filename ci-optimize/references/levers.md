# Levers

Each lever: how it shows in the measures, how to confirm it, the shapes of a fix, and what proves the fix. Levers that remove work come first.

## Superseded runs

- **Shows as** several runs of one branch or pull request overlapping after quick pushes.
- **Confirm** in the run list: runs of one ref whose time ranges overlap, the older ones finishing after a newer push.
- **Fix** by cancelling the older run of the same ref when a newer one starts (a concurrency group with cancel-in-progress, or the provider's equivalent). Keep the main branch's runs, whose results gate deploys.
- **Proof**: cancelled runs in the list, and billed minutes per pull request.

## Re-testing identical content

- **Shows as** a push to the main branch re-running checks right after a pull request passed them.
- **Confirm** by comparing the pushed commit's tree (`git rev-parse <sha>^{tree}`) with the tree a green run actually checked out. Where pull requests test a merge ref, the run API may report the head commit; its tree equals the tested one when the branch already contained its base.
- **Fix** by skipping when a green run proves that exact tree went through the real checks: match the tree, then require the checks step itself succeeded, since a run can end green with that step skipped. A merge queue, where the repository's owner type and plan offer one, makes the landed tree the tested one, so the push run can go; it moves the second run before the merge rather than removing it.
- **Proof**: the skip run logs which run it trusted, and its duration drops to setup.

## Work the change does not affect

- **Shows as** every job running on a change confined to one area.
- **Confirm** by mapping the changed paths of recent merges to the jobs that could fail on them.
- **Fix** with path filters or the build tool's affected-graph query. A required check that never starts stays pending and blocks the merge: for a required check, decide inside the job and report success with a skip line.
- **Proof**: jobs per change on a documentation-only change and on a one-package change.

## Duplicated checks

- **Shows as** the same tool appearing twice in one change's logs, in two jobs or twice in one job.
- **Confirm** by reading each workflow and the scripts its steps call: a step that repeats what a script chain already runs, a job re-running a check another required job owns.
- **Fix** with one owner per check: split the script chain so the CI variant drops the part another required job covers. Check the owner job always runs when the dropped part would have (no paths filter or mode that skips it).
- **Proof**: the check runs once per change; the job lost its duration.

## Oversized scope

- **Shows as** a heavy pass (type-aware lint, type check, coverage) much slower or hungrier in memory than its input suggests.
- **Confirm** by timing the pass with and without the suspect inputs: generated declarations, vendored code, fixtures, build output pulled in by a config include.
- **Fix** by leaving those inputs out of the heavy pass while a cheaper check still covers them. Mirror the exclusion everywhere the scope is declared (CI, pre-commit hooks, editor config).
- **Proof**: the pass's time and memory peak in CI, with its conditions.

## Expensive checks before cheap ones

- **Shows as** a long job failing on something a seconds-long check would have caught (format, lint, a type error).
- **Fix** by running the cheap checks first and gating the expensive jobs behind them, so a red change stops early.
- **Proof**: minutes spent on changes that failed a cheap check.

## A cache that never pays

- **Shows as** the same regeneration every run (images, compiled artifacts, build outputs, downloaded toolchains) in a fresh workspace; or a cache whose key changes every run; or a save and restore slower than the rebuild.
- **Confirm** with the regeneration count in the log, an empty cache directory at job start, the cache key across runs, and the save and restore times against the rebuild time.
- **Fix** by persisting the tool's own cache directory (a compiler cache, a build-system cache, container layers) in the runner's cache or the provider's cache, keyed on its real inputs. Trusted jobs on the main branch write it; pull requests read it. Know how the cache expires, and whether the tool ever prunes it.
- **Proof**: the run after a trusted job reports reuse (hit count against total), and the step's time drops by more than the save and restore cost.

## Serial independent work

- **Shows as** a script awaiting independent tool invocations one after another, each on its own inputs; or one long test job.
- **Confirm** by reading the loop and comparing the sum of the parts with the slowest part.
- **Fix** by running the parts concurrently, or sharding a long test suite across jobs. Both cut wall time; sharding raises billed minutes by each shard's setup. On shared CI cores the gain is smaller than locally: measure it in CI before claiming it.
- **Proof**: the step's or the pipeline's duration in CI against its slowest part, with its conditions, and the billed minutes.

## Hung and flaky jobs

- **Shows as** jobs that run to the provider's maximum time, or checks that pass on a re-run with no change.
- **Confirm** with the duration distribution (a cluster at the time limit) and re-run counts per check.
- **Fix** with a timeout per job near its normal duration times a margin, and by fixing or quarantining the flaky test instead of re-running the job.
- **Proof**: no run at the time limit; re-runs per week.

## Contention

- **Shows as** durations or queue times that swing with the number of concurrent jobs, for example during bursts of bot merges: high CPU wait and load on self-hosted runners, jobs queued against the plan's concurrency limit on hosted ones.
- **Confirm** as [measure.md](measure.md) describes: CPU wait close to the gap between low-load and high-load runs, or queue time that tracks the number of jobs in flight.
- **Fix** by running fewer jobs per change (the levers above), by rate limits and schedules on the bots that open and merge pull requests, by concurrency groups that serialize what need not run in parallel.
- **Proof**: CPU wait (self-hosted) or queue time (hosted) per job during the next burst.

## Setup overhead

- **Shows as** checkout, install and toolchain steps costing a large share of a short job.
- **Confirm** by the setup steps' share of the job across runs, and the cache hit or download lines in their logs.
- **Fix** with dependency and toolchain caches, pre-provisioned runners, a shallow checkout and no large-file fetch where the job needs no history, or deciding whether the job must run before installing anything.
- **Proof**: the setup steps' duration and hit count in CI on the run after a trusted job.
