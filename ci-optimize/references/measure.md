# Measuring CI time

## Sources, cheapest first

- **The provider's run API**: job and, where the provider records them, step start and end times, conclusions, queue time, the job graph.
- **Job logs**: every timing line a tool prints (bundler phases, `Done in`, per-file progress, cache hit or regeneration lines). Where the provider stamps every log line with a time, the gaps between lines time a tool that prints no timings of its own. Fetch logs through the API; they often carry terminal escape codes, so strip those before grepping.
- **The tool's own timing flag** (`--timing`, a verbose or profile mode) when the log is too coarse.
- **Runner host metrics** on self-hosted runners: CPU pressure (time a process waited for a core), hypervisor steal, memory peak, load. They separate a job's own cost from its neighbours'.
- **Local timing**: the same command on the same inputs, several runs, with a benchmarking tool if one is installed or the shell's `time`.

## Intrinsic cost or contention

- On self-hosted or shared runners, group runs of one job by the host's load. The low-load runs give the intrinsic cost; the gap to high-load runs is contention. CPU wait close to that gap means the job waited for cores: other jobs on the same host, or its own parallel steps outnumbering the cores.
- On hosted runners each job gets its own machine: contention shows as queue time against the plan's concurrency limit, and the spread comes from runner image updates, network downloads and the cache service.
- Compare before and after only between runs under similar conditions, and state them.

## Regressions

- Sort one job's runs by time and look for a step change in duration or memory peak under similar conditions. A memory jump is a strong signal: something new entered a heavy tool's input.
- Map the step to the commit range between the last fast run and the first slow one, then read that diff for what grew the input (a generated file, a vendored tree, a new config include).
- Confirm locally: the tool on its full input, then without the suspect. The difference is the marginal cost.

## Parallel work

A parallel step or job graph costs its critical path: shortening the slowest branch gains only down to the next slowest. Parallelism cuts wall time and leaves billed minutes the same or higher.

## Caches

A cache step that succeeds may restore nothing. Count the tool's own hit and miss lines (`reused`, `restored`, `hit`, versus `generating`, `download`) in the log, check the cache directory's size at the start of the job, and time the save and restore steps themselves.
