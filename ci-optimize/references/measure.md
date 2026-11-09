# Measuring CI time

## Runner minutes

- **Hosted runners**: the provider's usage or billing report where it has one. Otherwise each job's duration rounded as the provider bills it (GitHub Actions rounds each job up to the whole minute), times its runs, weighted by the rate of its runner class (a macOS or a larger runner costs several Linux minutes).
- **Self-hosted runners**: the minutes a job holds a runner, its queue time aside. Nothing is billed, but a held runner is one the next job waits for.

## Sources, cheapest first

- **The provider's run API**: job and, where the provider records them, step start and end times, conclusions, queue time, the job graph. Filter by conclusion before taking a median: failed and cancelled runs stop early and drag it down.
- **Job logs**: every timing line a tool prints (bundler phases, `Done in`, per-file progress, cache hit or regeneration lines). Where the provider stamps every log line with a time, the gaps between lines time a tool that prints no timings of its own: [`log_phases.py`](../scripts/log_phases.py) turns marker lines into phase durations. Fetch logs through the API; they often carry terminal escape codes, which some CLIs refuse to print without a flag (`gh api --allow-escape-sequences`), so strip them before grepping.
- **The test runner's per-test timing** (a JUnit report, a durations flag): a suite's time is often one slow test, and a test whose duration far exceeds its CPU time is waiting on a clock.
- **The tool's own timing flag** (`--timing`, a verbose or profile mode) when the log is too coarse.
- **System call counts**, where `strace` exists: `strace -f -c -e trace=execve <command>` counts the processes a command starts (thousands name fork-heavy work), and a histogram of the paths in `strace -f -e trace=%file,getdents64` shows which trees a tool walks.
- **Runner host metrics** on self-hosted runners: CPU wait (the kernel's CPU pressure: time a process waited for a core), hypervisor steal, memory peak, load. They separate a job's own cost from its neighbours'. Without a metrics history, have the job log its host's load and CPU wait at start and end.
- **Local timing**: the same command on the same inputs, several runs, with a benchmarking tool if one is installed or the shell's `time`.
- **Without the run API** (no token, no network, a provider without one): ask the user for access or an export of recent runs, and meanwhile label local timings local-only.

## Measuring locally

- Reproduce the CI's file layout first: installed dependencies as real directories rather than links, build output where CI puts it. A tool that walks the tree runs at another speed, or fails another way, in a different layout.
- Pin the command to the runner's core count (`taskset -c 0-3` for 4 cores; a container runner's limit is in its cgroup's `cpu.max`). A workstation with more cores hides both the gain of parallel work and its contention.
- Alternate the before and after runs, record the machine's load beside each, and leave the measured tree untouched while they run.
- Keep the full output of every run: an intermittent failure is gone once a filter has dropped its lines.

## Intrinsic cost or contention

- On self-hosted or shared runners, group runs of one job by the host's load. The low-load runs give the intrinsic cost; the gap to high-load runs is contention. CPU wait close to that gap means the job waited for cores: other jobs on the same host, or its own parallel steps outnumbering the cores.
- On hosted runners each job gets its own machine: contention shows as queue time against the plan's concurrency limit, and the spread comes from runner image updates, network downloads and the cache service.
- Compare before and after only between runs under similar conditions, and state them.

## Regressions

- Sort one job's runs by time and look for a step change in duration, CPU time or memory peak under similar conditions. A memory jump is a strong signal: something new entered a heavy tool's input.
- Map the step to the commit range between the last fast run and the first slow one, then read that diff for what grew the input (a generated file, a vendored tree, a new config include) or what joined the job (new tests, a new tool).
- Compare the phase breakdown of a low-load run on each side of the change: the phases that grew name the suspect.
- Confirm locally: the tool on its full input, then without the suspect. The difference is the marginal cost.

## Parallel work

- A parallel step or job graph costs its critical path: shortening the slowest branch gains only down to the next slowest. Parallel jobs cut wall time and leave runner minutes the same or higher, each adding its setup; parallel steps inside one job cut both, while the job's cores are free.
- A job's CPU time divided by its cores is the floor parallel work can reach; a CPU time close to the wall time means the job runs on one core. When several jobs share a host, each job's parallel workers multiply against the others': the host's CPU, not the job's shape, then sets the wall time.
- Once items run concurrently, the gaps between log lines no longer time them: have the runner print each item's duration.

## Caches

A cache step that succeeds may restore nothing. Count the tool's own hit and miss lines (`reused`, `restored`, `hit`, versus `generating`, `download`) in the log, check the cache directory's size at the start of the job, and time the save and restore steps themselves.
