---
name: deterministic-driver
description: >
  Use when building, operating, or resuming a deterministic Workflow() driver
  for a large autonomous multi-phase task — a full rewrite, a migration, a
  long audit. Triggers: design or implement a workflow driver script, translate
  a .agents/loops/*.md runner spec (looper output) into a Workflow() script,
  design phase gates or pause/resume mechanics, handle a paused driver run
  (retryStep bump, human arbitration, mutation suspected), harden a driver
  against non-deterministic agent behavior, or wrap an external CLI
  (e.g. codex exec) inside a workflow agent.
license: MIT
compatibility: Claude Code Workflow() scripts (deterministic JS driver,
  agent()/pipeline()/parallel(), resumeFromRunId prefix cache). Downstream of
  the looper skill's .agents/loops/*.md specs.
metadata:
  version: "0.1.0"
  author: "bengous"
---

# Deterministic Driver

Design, build, and operate a deterministic Workflow() driver: a plain-JS
script that owns every orchestration decision, while LLM agents are reduced
to schema-constrained executors, probes, and reviewers. The script judges
exit codes, fingerprints, and verdicts; no agent ever decides control flow.

Distilled from real runs — a 10-phase rewrite (bookmarker → Rust,
2026-07-15) and the P0 slice of a compositor-mode driver (hyprpilot
--isolated, 2026-07-25): every rule below was paid for by an incident or
measured on transcripts.

## When to use

- Build a driver for a big autonomous task (rewrite, migration, audit) with
  phase gates, commits, and human escalation.
- Translate a looper spec (`.agents/loops/*.md`) into a Workflow() script.
- A driver run paused and you must resume it correctly.
- Review or harden an existing driver.

When NOT to use: one-shot fan-outs, research workflows, anything without
durable state or resume requirements — plain Workflow() patterns suffice.

## Architecture in one paragraph

The driver is invoked by `scriptPath` (never by name from inside the target
repo) and lives OUTSIDE the target repo whenever the repo is watched by
fingerprint — otherwise every operator edit mid-run poisons the protected-path
hash. Durable state lives in **git** (one driver commit per phase); the
prefix cache (`resumeFromRunId`) is a disposable accelerator. Agents come in
three shapes: mechanical executors (run listed commands verbatim, report
results by schema), read-only reviewers (fingerprint-sandwiched), and external
CLI wrappers (detached launch + blocking waits). Pauses are typed exceptions
carrying a mandatory `retryStep` and a self-sufficient `howToResume` — the
operator must never need to reread the script to handle a standard pause.

Code for every mechanism: [references/patterns.md](references/patterns.md).
Ready-to-instantiate skeleton: [references/template.js](references/template.js).
Full real-world instance (every mechanism assembled — frozen snapshot of the
driver that ran a 10-phase Rust rewrite):
[examples/rust-rewrite-runner.js](examples/rust-rewrite-runner.js). Copy
MECHANISMS from the example, never its agent economy: it predates rule 7's
batching mandate and spends roughly twice the agents the same guarantees
need (measured on a derived driver, retro 2026-07-25). Instantiate from the
template.

## Design rules

1. **Git is the only durable state; the prefix cache is disposable.**
   A committed-check at the start of every phase (`git log` subject match)
   makes cold restart cheap: committed phases are skipped, only the front
   phase replays. Default to cold restart at inter-phase pauses; reserve
   prefix resume for intra-phase pauses. A cold restart costs a handful of
   probe agents — often less than a resume that replays a review cycle.

2. **No "if X is available… otherwise" branch in any agent prompt.**
   That is an orchestration decision delegated to the LLM. Measured result:
   with a conditional "use the code-review skill if available", agents
   invoked it in 4 phases out of 9 and never even mentioned it in the others.
   Impose the methodology (mandatory invocation), require invocation failure
   to be reported explicitly, never allow a silent fallback. Verify required
   tools by proof (transcript, modelUsage), not by trust.

3. **Prompt rules in terms of OPERATIONS, never intentions.**
   "cp to scratch allowed, opening the file forbidden" — not "read-only".
   Trap that forced this rule: opening a SQLite WAL database *for reading*
   creates/touches the `-shm` file, so "read-only" is physically impossible
   to honor on a stat-hashed profile directory. An intention rule the agent
   cannot satisfy to the letter will be violated; an operation rule is
   checkable and followable.

4. **Both halves of a gate need the same temporal coverage.**
   A gate has a detection half (e.g. `pgrep`: browsers closed) and an
   interpretation half (e.g. stat-hash pair: any change is the gate's
   fault). If detection runs only at window start while interpretation
   spans the whole window, the interpretation works from an assumption
   detection no longer guarantees (TOCTOU). Check detection at both ends
   of the window. Also decide what the gate actually protects: a
   browsers-closed pgrep is a *signal-quality* device (makes the stat-hash
   attributable), not the anti-corruption defense — that is the fail-closed
   stat-hash pair itself.

5. **Always bump the indicated retryStep — even when a prompt edit seems
   sufficient.** `retryStep` designates the OBSERVATION to replay, not the
   faulty work: replaying only the failed step can leave a before/after
   fingerprint pair incoherent (before cached, after live) and loop the
   pause. One real resume worked without a bump purely by accident of
   construction (an unrelated prompt change happened to break the cache
   earlier); gated differently it would have replayed the cached failure
   forever.

6. **Gate mid-run prompt fixes by phase; de-gate everything on a fresh run.**
   The first agent() call whose prompt changes invalidates the whole cache
   suffix. Mid-run, wrap any prompt hardening in `p >= N` so already-played
   steps stay byte-identical. Phase gating is a cache artifact, not a design
   feature: when writing a new driver or restarting cold, every hardening
   belongs in the base prompt of every applicable phase. Fix prompt defects
   EARLY — the further the resume front advances, the more a global prompt
   change equals a cold restart.

7. **The fixed per-agent overhead dominates cost — batch, don't multiply.**
   ~27-54K tokens of boot (system prompt, instruction files, skills, MCP
   schemas) per agent, whether it runs 2 commands or 20. Measured on a real
   P0: 16 of 29 agents were bookkeeping probes producing 6% of output
   tokens. The operative rules, none optional:
   - ONE mechanical agent per contiguous mechanical sequence. A probe, a
     gate run, and a file listing with no non-mechanical step between them
     are one agent, not three.
   - Fingerprint-sandwich per contiguous READ-ONLY BLOCK, not per agent.
     Chain: the after of one block is the before of the next; never two
     probes back-to-back with nothing between them. Per-agent attribution
     is not worth a probe — the decision is "pause" either way and the
     transcripts identify the culprit.
   - Mechanical tier = the cheapest model that reliably copies command
     output verbatim, never the frontier model.
   - Write the agent budget per phase in the mapping-decisions block before
     the driver may run, and log() the actual count per phase. A cost
     nobody sees is a cost nobody debates. Nominal target: ~6 agents per
     phase, ~3 more per correction round.
   And probe every external wrapper mechanism BEFORE launch — the one
   unprobed path (a CLI run exceeding the Bash timeout) killed a run in
   phase 1.

8. **Know your platform traps.**
   - `pgrep -x` matches the kernel comm, truncated to 15 chars: a pattern
     of 16+ chars can NEVER match. Truncate patterns to 15 or list real
     comm values.
   - Schema-enforced agents die if they end their turn without output —
     they can never "wait" by ending the turn. Long commands: detached
     launch (`setsid nohup`, exit code to a known file) + repeated blocking
     Bash waits under the 600 s tool cap. Give mechanical executors a
     relief valve: a harness-backgrounded command is reported as a clean
     failure (`exitCode: -1`), turning a run death into a recoverable pause.
   - `resumeFromRunId` IGNORES `args`: the file's top-of-script constants
     are the ONLY resume channel.

9. **Adversarially fact-check any finding about the driver itself** with a
   clean-context third party before recording it — and fact-check the
   refuter's proposed fixes too. Real score on 3 findings: 2 corrected
   (overstated impact, wrong threat model), 1 worse TOCTOU discovered, and
   one refuter claim ("JS-only change, therefore cache-compatible") was
   itself false.

10. **Decide at design time which human gates are delegable to the
    orchestrator, and write it down.** An explicit delegation note turned
    47 minutes of dead waiting into an automatic unblock.

11. **Warn the operator: edit the target repo only between phases.** The
    driver cannot distinguish a legitimate user edit from an agent mutation
    inside a fingerprint window — both pause the run. Before resuming after
    any mutation-suspected pause, establish which one it was (diff against
    backup, integrity checks), then acknowledge explicitly through the
    resume channel so the driver re-baselines on purpose, not by accident.
    Corollary: fingerprint only what the run OWNS. A protected-path set
    covering a parent directory other sessions legitimately write to
    (a shared specs/ dir, `.claude/` which the harness itself touches)
    pauses every phase on unrelated writes — protect the frozen files
    themselves, not their directory (real pause: an unrelated spec landing
    in the watched dir mid-verification).

12. **Preflight must prove the gate GREEN on the virgin tree — run it,
    don't inventory it.** Checking that tools exist (`shellcheck
    --version`) while never running them on the tree lets a pre-existing
    red gate masquerade as the first slice's failure. Measured cost of
    missing this: one full implementation attempt burned on an
    out-of-mandate fix, an escalation neither mechanical verdict could
    unblock (keep the fix → commit-allowlist pollution; revert → red
    gate), three human round-trips. Corollary: a phase that must write
    outside its allowlist to green the gate is a configuration dead-end —
    detect it at preflight, not at commit time.

13. **A human decision is terminal — carry it everywhere, let no channel
    override it.** Two halves, both violated in one real loop: (a) every
    downstream agent that could reopen a question must receive ALL human
    decisions — a ledger carrying only the REQUIRED_FIX entries showed the
    verifier `humanArbitration: []`, which it read as "no decision was
    ever made" and reopened a closed escalation; (b) no verdict channel,
    including an escape hatch like PAUSE, may bypass the control flow that
    honors human decisions — otherwise the pause loops forever on a
    question the human already answered.

14. **Scale the ceremony to the phase's stakes; bound review scope by the
    commit allowlist.** One rigor level for all phases overpays the
    low-stakes ones: a doc-only slice got the full review→fact-check→
    arbitration→verification chain — 6 agents, 204 turns — for 2 minor
    fixes and 7 findings that were structurally unfixable (scratchpad
    scripts outside the repo, already executed). Declare a per-phase
    review policy (full chain for code slices, single light review for doc
    slices), and tell the review its scope IS the phase allowlist: what
    cannot be committed cannot be fixed — findings there are operator
    notes, not phase blockers.

## Operating a paused run — checklist

1. Read the pause payload: `reason`, `detail`, `retryStep`, `howToResume`.
   It must be self-sufficient; if it is not, that is a driver bug to fix.
2. Treat the cause (close browsers, restore state, decide arbitration…).
3. Edit the script's top constants — the only resume channel:
   - `RETRY_TOKENS[retryStep] += 1` — ALWAYS when the pause carries a
     retryStep, rule 5. No exceptions for "the prompt change will break
     the cache anyway". When `retryStep` is null (pure JS gate:
     arbitration, acknowledgment constants), there is no observation to
     replay — the edited constant is the entire channel; bump nothing.
   - `HUMAN_ARBITRATION[findingId] = {verdict, requiredOutcome?}` for
     escalated findings.
   - mutation-suspected pauses: record the verification you performed in
     the acknowledgment constant (e.g. `MUTATION_ACK`) so the driver
     re-baselines explicitly.
4. Any prompt fix riding along must be phase-gated (rule 6).
5. `Workflow({scriptPath, resumeFromRunId})`. `args` will be ignored.
6. Inter-phase pause with a messy cache? Prefer cold restart: new run,
   committed-check skips finished phases (rule 1).

## Build checklist

- [ ] Spec read (looper `.agents/loops/*.md` or equivalent); every gate
      mapped to a command, a fixed-path deliverable script, or a review
      judgment — and the mapping decisions documented at the top of the
      script (paid once at first read, available at every reopen).
- [ ] CONFIG section isolates everything project/machine-specific; the body
      references only CONFIG.
- [ ] Resume constants (`RETRY_TOKENS`, `HUMAN_ARBITRATION`, acknowledgment
      constants) empty/null at the top, above CONFIG.
- [ ] Every pause has a `retryStep` pointing at the observation to replay,
      and a hint saying exactly what to edit.
- [ ] Fingerprint sandwich around every contiguous READ-ONLY BLOCK of
      non-mechanical calls; consecutive probes chained, never doubled
      (rule 7).
- [ ] Preflight RUNS the full deterministic gate on the virgin tree
      (rule 12) — a tool inventory alone is not a preflight.
- [ ] Agent budget per phase written in the mapping-decisions block;
      actual count logged per phase (rule 7).
- [ ] Per-phase review policy declared (full chain / light / none) and
      review scope bounded to the phase allowlist (rule 14).
- [ ] Human decisions travel whole: the ledger carries every verdict
      including DISMISSED, and no verdict channel bypasses the
      human-arbitration filter (rule 13).
- [ ] Resume constants classified prompt-visible (`RETRY_TOKENS` — by
      design) or driver-only (everything else); no driver-only constant
      feeds a value that appears in any prompt — widening one mid-run
      would invalidate the cache and regenerate the finding IDs your
      resume keys reference.
- [ ] Commits judged on observed state (head moved + worktree clean), never
      on reported exit codes; committed-check at every phase start.
- [ ] External CLI wrappers probed for: >600 s runs, effective model/effort
      reporting, session resume semantics (per-flag inheritance!), unknown
      session id behavior.
- [ ] No conditional tool/skill branches in prompts (rule 2); rules stated
      as operations (rule 3).
- [ ] Human-gate delegation decided and written down (rule 10).
- [ ] Script located outside the target repo; operator warned about
      mid-phase edits (rule 11).

## Common mistakes

| Mistake | Consequence |
|---|---|
| Resuming without bumping retryStep | Cached failure replays; pause loops forever |
| Prompt fix without phase gating mid-run | Cache invalidated from that step; effectively a cold restart you didn't choose |
| "Read-only" instruction on a WAL SQLite path | Agent touches `-shm` by merely opening it; mutation pause with wrong attribution |
| `pgrep -x` pattern longer than 15 chars | Dead entry, never matches; blind spot in the gate |
| Trusting reported exit codes for commit decisions | Double-commit or missed commit on executor misreport |
| Letting a schema-enforced agent wait by ending its turn | Agent killed by StructuredOutput enforcement; whole run dies |
| Resume channel via `args` | Silently ignored on resume; run replays original args |
| Live guard at the top of the script | Invalidates the entire prefix cache; re-verify where it matters (start of destructive phases) instead |
| One constant feeds both a prompt and the commit command | Widening it mid-run invalidates the review cache; regenerated finding IDs orphan the resume keys |
| PROTECTED covers a parent directory other sessions write to | Every concurrent legitimate write pauses the run; protect the frozen files, not their directory |
| Verdict escape hatch (PAUSE) checked before the human-arbitration filter | A human DISMISSED is silently overridden; the pause loops on an answered question |
| Ownership inferred from "absent from the start snapshot" | Anything created during the window also matches — including windows/resources of concurrent users; prove ownership by lineage (a PID you spawned), never by set subtraction |
