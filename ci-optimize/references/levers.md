# Levers

Each lever: how it shows in the measures, how to confirm it, the shapes of a fix, and what proves the fix. Levers that remove work come first.

## Work that should not run

### Superseded runs

- **Shows as** several runs of one branch or pull request overlapping after quick pushes.
- **Confirm** in the run list: runs of one ref whose time ranges overlap, the older ones finishing after a newer push.
- **Fix** by cancelling the older run of the same ref when a newer one starts (a concurrency group with cancel-in-progress, or the provider's equivalent). Keep the main branch's runs, whose results gate deploys.
- **Proof**: cancelled runs in the list, and runner minutes per pull request.

### Bot rebase runs

- **Shows as** many runs on few refs: a dependency bot rebases its pull requests each time the main branch moves, each rebase triggers the pipeline, and the waves cancel each other, often while those pull requests wait for a human.
- **Confirm** by counting runs per ref, per event and per actor over the measured window, and matching their start times to pushes on the main branch. Read the bot's rebase setting and the branch rules: a rule that requires branches up to date makes a bot rebase every pull request that falls behind.
- **Fix** by rebasing only what the bot merges itself, and by giving the bot hours to work in. For Renovate: `rebaseWhen: "conflicted"` globally and `"behind-base-branch"` on the automerge rules; a `schedule` with its `timezone`, plus `updateNotScheduled: false`, without which existing branches keep updating outside the schedule. A pull request that waits for a human is then rebased once, before its merge.
- **Proof**: runs per bot pull request, and the hours they start at, over the next week.

### Re-testing identical content

- **Shows as** a push to the main branch re-running checks right after a pull request passed them.
- **Confirm** by comparing the pushed commit's tree (`git rev-parse <sha>^{tree}`) with the tree a green run actually checked out. Where pull requests test a merge ref, the run API may report only the head commit, so record the checked-out tree in the run itself: a step name can carry it (GitHub Actions evaluates `steps` outputs in a step's `name`), read back through the jobs API where artifacts are out of reach. Count how often the main branch stays still between a pull request's run and its merge: on a busy branch the trees rarely match, and the lever pays mostly for bot merges in quiet hours.
- **Fix** by skipping when a green run proves that exact tree went through the real checks. The proof is a step that runs only after the checks step ran and passed, so neither a skipped checks step nor a skip that reports success produces it. A merge queue, where the repository's owner type and plan offer one, makes the landed tree the tested one, so the push run can go; it moves the second run before the merge rather than removing it.
- **Proof**: the skip run logs which run it trusted, and its duration drops to setup. To watch it fire, merge while the main branch is unchanged since the pull request's run.

### Work the change does not affect

- **Shows as** every job running on a change confined to one area.
- **Confirm** by mapping the changed paths of recent merges to the jobs that could fail on them. A pre-push hook or the build tool may already route paths to checks.
- **Fix** with path filters, the build tool's affected-graph query, or the repository's existing router. Where the provider leaves a required check that never starts pending (GitHub Actions does with path filters), decide inside the job and report success with a skip line. Hold these invariants:
  - The base is the newest main-branch commit a run passed on, for pushes and pull requests alike; a run that passed because the routing found nothing to check counts, since the scheduled run covers what the routing misses. A base taken from the previous push leaves a cancelled or failed run's changes unvalidated, and a pull request diffed against a broken main branch can turn the next run green over the break.
  - A change to the CI configuration, the toolchain pins or the router itself runs everything.
  - A scheduled run validates everything, to catch what the routing misses, in its own concurrency group so it never cancels a push run.
  - The repository's agent docs say that a check the routing misses waits for that scheduled run.
- **Proof**: the plan the job logs and its duration, on a documentation-only change and on a one-package change; the scheduled run green.

### Duplicated checks

- **Shows as** the same tool appearing twice in one change's logs, in two jobs or twice in one job.
- **Confirm** by reading each workflow and the scripts its steps call: a step that repeats what a script chain already runs, a job re-running a check another required job owns.
- **Fix** with one owner per check: split the script chain so the CI variant drops the part another required job covers. Check the owner job always runs when the dropped part would have (no paths filter or mode that skips it).
- **Proof**: the check runs once per change; the job lost its duration.

### Oversized scope

- **Shows as** a heavy pass (type-aware lint, type check, coverage) much slower or hungrier in memory than its input suggests.
- **Confirm** by timing the pass with and without the suspect inputs: generated declarations, vendored code, fixtures, build output pulled in by a config include.
- **Fix** by leaving those inputs out of the heavy pass while a cheaper check still covers them. Mirror the exclusion everywhere the scope is declared (CI, pre-commit hooks, editor config).
- **Proof**: the pass's time and memory peak in CI, with its conditions.

## Work that runs slower than it needs

### Waste inside tests

- **Shows as** a suite slower than its work: a test that sleeps out a timeout, fixtures built by starting one process per record, a duration far above the suite's CPU time.
- **Confirm** with the per-test timing and the process count ([measure.md, Sources](measure.md#sources-cheapest-first)).
- **Fix** by asserting the setting instead of waiting out the timeout it sets, and by building fixtures in one process.
- **Proof**: the test's duration in the CI log, and the test still failing when the setting it guards is broken on purpose.

### Startup paid per call

- **Shows as** a tool called once per file, template or package, each call paying the same startup: loading a source tree, a runtime, an index.
- **Confirm** by timing a call that does nothing on the same inputs, times the number of calls; the path histogram ([measure.md, Sources](measure.md#sources-cheapest-first)) shows what the startup walks, such as dependency trees or build output.
- **Fix** with one call for all inputs (the tool's multi-file or include mode), or a narrower root for the tool to load.
- **Proof**: the step's duration, and per-input results identical to the calls they replace.

### Expensive checks before cheap ones

- **Shows as** a long job failing on something a seconds-long check would have caught (format, lint, a type error).
- **Fix** by running the cheap checks first and gating the expensive jobs behind them, so a red change stops early.
- **Proof**: minutes spent on changes that failed a cheap check.

### A cache that never pays

- **Shows as** the same regeneration every run (images, compiled artifacts, build outputs, downloaded toolchains) in a fresh workspace; or a cache whose key changes every run; or a save and restore slower than the rebuild.
- **Confirm** with the regeneration count in the log, an empty cache directory at job start, the cache key across runs, and the save and restore times against the rebuild time.
- **Fix** by persisting the tool's own cache directory (a compiler cache, a build-system cache, container layers) in the runner's cache or the provider's cache, keyed on its real inputs. Trusted jobs on the main branch write it and pull requests read it, so code under review cannot plant what the main branch's runs later restore. Know how the cache expires, and whether the tool ever prunes it.
- **Proof**: the run after a trusted job reports reuse (hit count against total), and the step's time drops by more than the save and restore cost.

### Serial independent work

- **Shows as** a script awaiting independent tool invocations one after another, each on its own inputs; or one long test job.
- **Confirm** by reading the loop and comparing the sum of the parts with the slowest part. Before running parts together, audit what they share: fixed temporary paths, the home directory, global config, ports, locks, waits bound to the clock, and the working tree itself (a part that walks the tree while another writes there). Build output inside the tree races any tool that walks it: keep it outside, or keep those parts apart. The audit reads every part, so delegate it with this list as its brief, or run that brief yourself where the harness has no subagents.
- **Fix** by running the parts concurrently, or sharding a long test suite across jobs. Both cut wall time; sharding raises runner minutes by each shard's setup. The gain holds only while the host has free cores (measure.md, Parallel work): measure it in CI before claiming it, and keep each part's duration in the log.
- **Proof**: the step's or the pipeline's duration in CI against its slowest part, with its conditions, and the runner minutes.

### Setup overhead

- **Shows as** checkout, install and toolchain steps costing a large share of a short job.
- **Confirm** by the setup steps' share of the job across runs, and the cache hit or download lines in their logs.
- **Fix** with dependency and toolchain caches, pre-provisioned runners, a shallow checkout and no large-file fetch where the job needs no history, or deciding whether the job must run before installing anything.
- **Proof**: the setup steps' duration and hit count in CI on the run after a trusted job.

## Runs that fail or wait

### Hung and flaky jobs

- **Shows as** jobs that run to the provider's maximum time, or checks that pass on a re-run with no change.
- **Confirm** with the duration distribution (a cluster at the time limit) and re-run counts per check.
- **Fix** with a timeout per job near its normal duration times a margin, and by fixing or quarantining the flaky test instead of re-running the job.
- **Proof**: no run at the time limit; re-runs per week.

### Contention

- **Shows as** durations or queue times that swing with the number of concurrent jobs, for example during bursts of bot merges: high CPU wait and load on self-hosted runners, jobs queued against the plan's concurrency limit on hosted ones.
- **Confirm** as [measure.md](measure.md) describes: CPU wait close to the gap between low-load and high-load runs, or queue time that tracks the number of jobs in flight.
- **Fix** by running fewer jobs per change (the levers under Work that should not run, bot rebase runs first), by concurrency groups that serialize what need not run in parallel, and by fewer parallel workers per job when several jobs share a host.
- **Proof**: CPU wait (self-hosted) or queue time (hosted) per job during the next burst.
