# Deterministic driver — code patterns

Extracted from a real 10-phase driver run (rust-rewrite-runner,
wf_3b55d1fb-940, 2026-07-15). Snippets are genericized; `CONFIG.*` marks
values that belong in the CONFIG section. A full assembled skeleton is in
[template.js](template.js); the complete real instance is in
[../examples/rust-rewrite-runner.js](../examples/rust-rewrite-runner.js).

## 1. File-based resume channel

`resumeFromRunId` ignores the `args` parameter (validated by probe) — edited
top-of-file constants are the ONLY channel from operator to resumed run.

```js
// --- RESUME CHANNEL: constants edited by the operator between resumes ---
// Bumping a step's token re-runs THAT step live plus everything after it;
// everything before replays from cache. The pause names the exact stepId.
const RETRY_TOKENS = {}            // e.g. { 'p3.val.1': 1 }

// Human decisions on escalated findings, applied by the DRIVER on resume.
const HUMAN_ARBITRATION = {}       // e.g. { 'F-12': { verdict: 'DISMISSED' } }

// Post-mutation acknowledgment: after a mutation-suspected pause, resume
// requires an entry attesting the verification performed; the driver then
// re-baselines its watched-state reference explicitly.
const MUTATION_ACK = {}            // e.g. { 'p8.val.1': 'backup diffed clean, integrity_check ok, wal empty' }
```

Selective cache invalidation — the token is injected into the prompt of that
step ONLY, so bumping it invalidates just that step and its suffix:

```js
function rt(stepId) { return RETRY_TOKENS[stepId] || 0 }
function hdr(stepId) { return `[${CONFIG.NAME} step:${stepId} retry:${rt(stepId)}]` }
// Every agent prompt starts with hdr(stepId).
```

`HUMAN_ARBITRATION` and acknowledgment constants are read only by the JS (or
by steps at/after the resume front), so editing them never breaks the cache.

Corollary — classify every constant by visibility. `RETRY_TOKENS` is
prompt-visible BY DESIGN (that is its invalidation mechanism). Everything
else must stay driver-only: a constant that feeds both a prompt and a
mutating command (a phase allowlist used by the review-scope listing AND
the commit command, say) cannot be widened mid-run without invalidating the
review cache — and the regenerated finding IDs orphan the
`HUMAN_ARBITRATION` keys that were the whole point of the resume. If the
commit needs a wider list than the prompt, give the commit its own constant
(real fix: a separate `COMMIT_ALLOW_EXTRA`, consumed only by the commit
step).

Typed pause with mandatory retryStep — without it, the failure result would
replay from cache on resume and the pause would loop forever:

```js
class Pause {
  constructor(reason, detail, retryStep) {
    this.reason = reason; this.detail = detail; this.retryStep = retryStep
  }
}
// retryStep designates the OBSERVATION to replay (e.g. the `before`
// fingerprint of a pair), not merely the faulty work — otherwise the pair
// is incoherent on resume (before cached, after live).
function pause(reason, detail, retryStep) { throw new Pause(reason, detail, retryStep) }
```

Self-sufficient pause payload — the operator must never reread the script
for a standard pause:

```js
} catch (e) {
  if (e instanceof Pause) {
    return {
      status: 'paused', reason: e.reason, detail: e.detail,
      retryStep: e.retryStep || null,
      howToResume: e.retryStep
        ? `1) Treat the cause. 2) Edit this script: RETRY_TOKENS["${e.retryStep}"] = <current + 1>. `
          + `3) Workflow({scriptPath, resumeFromRunId}). NEVER resume without the bump: the cached `
          + `failure would replay and the pause would loop. The args parameter is ignored on resume — `
          + `the file is the only channel.`
        : `1) Treat the cause. 2) Edit the constant named in the hint (HUMAN_ARBITRATION / ...). `
          + `3) Workflow({scriptPath, resumeFromRunId}). These constants are read only by the driver: `
          + `the prior cache is preserved.`,
    }
  }
  throw e
}
```

Script placement: OUTSIDE the target repo (invoked by `scriptPath`). If it
lived under `<repo>/.claude/`, every constant edit mid-run would change a
protected path → the protected-content hash would diverge between cached and
live fingerprints → spurious mutation pauses.

## 2. Fingerprint sandwich (read-only enforcement)

No permissionMode can be imposed on workflow agents. Prevention is by prompt;
DETECTION is by fingerprint before/after every contiguous READ-ONLY BLOCK of
non-mechanical calls — never per agent (skill rule 7: per-agent sandwiching
made 16 of 29 agents of a real P0 pure bookkeeping).

```js
async function fingerprint(stepId, phaseName) {
  const prompt = [
    hdr(stepId),
    `Read-only probe for the driver. From ${CONFIG.REPO}, run exactly these commands and copy values verbatim:`,
    '- branch : git rev-parse --abbrev-ref HEAD',
    '- head : git rev-parse HEAD',
    '- indexSha256 : git diff --cached | sha256sum (first field)',
    '- statusSha256 : git status --porcelain=v2 --untracked-files=all | LC_ALL=C sort | sha256sum (first field)',
    '- diffSha256 : git diff | sha256sum (first field)',
    `- protectedSha256 : find ${CONFIG.PROTECTED.join(' ')} -type f -print0 2>/dev/null | LC_ALL=C sort -z | xargs -0 -r sha256sum | sha256sum (first field)`,
    MECH_RULES,
  ].join('\n')
  const f = await agent(prompt, { schema: FP_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH })
  if (!f) pause('fingerprint_agent_failed', { stepId }, stepId)
  if (f.branch !== CONFIG.BRANCH) pause('wrong_branch', { expected: CONFIG.BRANCH, got: f.branch }, stepId)
  return f
}

// afterStepId = the step to re-probe/re-run after human remediation.
function assertUnchanged(before, after, reason, phaseNum, afterStepId) {
  const same = before.head === after.head &&
    before.indexSha256 === after.indexSha256 &&
    before.statusSha256 === after.statusSha256 &&
    before.diffSha256 === after.diffSha256 &&
    before.protectedSha256 === after.protectedSha256
  if (!same) pause(reason, { phase: phaseNum, before, after }, afterStepId)
}

// Usage — the sandwich, per contiguous READ-ONLY BLOCK. Chain: the block's
// `before` is whatever probe you already hold (the post-validation one);
// the block's `after` is ONE bare fingerprint at the end. The review,
// fact-check and arbitration below share a single sandwich — never probe
// between two read-only agents with nothing else between them.
const before = valProbe /* fp from the validation step — already in hand */
const review = await agent(reviewPrompt, { schema: FINDINGS_SCHEMA, ...CONFIG.REVIEW })
const fc = await agent(factcheckPrompt, { schema: DISPOSITIONS_SCHEMA, ...CONFIG.REVIEW })
const arb = await agent(arbitrationPrompt, { schema: ARBITRATION_SCHEMA, ...CONFIG.REVIEW })
const after = await fingerprint(`p${p}.readonly.end`, phaseName)
assertUnchanged(before, after, 'readonly_block_mutated_repo', p, `p${p}.readonly.end`)
```

Block-level attribution is coarser than per-agent — accepted: the decision
is "pause" either way, and the per-agent transcripts identify the culprit
for free.

Empirically validated: caught a `-shm` mutation produced by a gate script
merely *opening* a WAL SQLite file for reading. Cost note: each probe pays
the full ~27-54K-token agent floor — hence block-level sandwiching above,
plus the merged probe (fingerprint + adjacent commands in ONE agent, see
template.js `probe()`). Mechanical tier: the cheapest model that copies
output verbatim, never the frontier model.

## 3. Anti-empty-gate exec (1:1 verbatim matching)

Mechanical executors run listed commands and report by schema. Two checks
guarantee a gate can never pass "empty": cardinality and verbatim match.

```js
const MECH_RULES = [
  'Strict rules:',
  '- You are a mechanical executor for the driver. NO interpretation, NO initiative.',
  '- Run nothing beyond the listed commands. Fix nothing. Retry nothing.',
  '- A failing command: record its result and move to the next.',
  '- Modify no files. Forbidden: git add/commit/push/reset/stash, rm, writes outside the repo.',
  '- Copy values VERBATIM from real outputs. No invented values.',
  '- Report ONE result per listed command, same order, command copied verbatim.',
].join('\n')

async function exec(stepId, phaseName, commands, context) {
  const prompt = [
    hdr(stepId),
    `Context: ${context}`,
    `From ${CONFIG.REPO}, run via Bash, in order, exactly these ${commands.length} commands:`,
    ...commands.map(c => `- ${c}`),
    MECH_RULES,
    // Relief valve: a harness-backgrounded timeout becomes a clean failure,
    // never an end-of-turn wait (StructuredOutput enforcement would kill the agent).
    'Timeouts: pass timeout=600000 to the Bash tool for each command. If a command hits this timeout and is moved to the background by the harness, do NOT wait for it and do not end your turn: report exitCode=-1 and outputTail="TIMEOUT_BACKGROUNDED" for it, then continue the list.',
    'For each command return: command (verbatim), exitCode, outputTail (last 60 lines max).',
  ].join('\n')
  const out = await agent(prompt, { schema: EXEC_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH })
  if (!out) pause('executor_agent_failed', { stepId }, stepId)
  // A gate never passes "empty": 1:1 command/result correspondence.
  if (out.results.length !== commands.length) {
    pause('executor_results_mismatch', { stepId, expected: commands.length, got: out.results.length }, stepId)
  }
  // Comparison modulo the `bash -c '...'` envelope: executors systematically
  // strip the wrapper when copying the command back (observed repeatedly).
  // The INNER command stays verbatim and 1:1 cardinality is still enforced.
  const normCmd = c => {
    const t = c.trim()
    const m = t.match(/^bash -c '([\s\S]*)'$/)
    return (m ? m[1] : t).trim()
  }
  const mismatched = out.results
    .map((r, i) => (normCmd(r.command) === normCmd(commands[i]) ? null : { index: i, expected: commands[i], got: r.command }))
    .filter(Boolean)
  if (mismatched.length) pause('executor_results_mismatch', { stepId, mismatched }, stepId)
  return out.results
}
```

Do not require fields the driver never reads (a hash of the output, say):
the only schema field with no decisional consequence is the one an executor
can invent for free. Every required field should feed a judgment.

Residual trust: exitCode values are still LLM-reported — which is why
critical decisions (commits) are judged on OBSERVED STATE (pattern 6).

## 4. External CLI wrapper (detached launch + blocking waits)

For a non-Claude implementer behind a CLI (e.g. `codex exec`). Two hard
constraints: a schema-enforced agent cannot end its turn to wait, and a
single Bash call caps at 600 s while the CLI may run for hours.

```js
function cliWrapperPrompt(stepId, resumeSessionId, instruction) {
  const work = `${CONFIG.CLI_WORK}/${stepId.replace(/[^A-Za-z0-9._-]/g, '_')}`
  const cmd = resumeSessionId
    // Probe your CLI's resume semantics per flag: codex `exec resume` inherits
    // the model but SILENTLY DROPS the reasoning effort (falls back to
    // config default) — so effort is re-passed explicitly on EVERY resume.
    ? `codex exec resume "${resumeSessionId}" -m ${CONFIG.CLI_MODEL} -c model_reasoning_effort=${CONFIG.CLI_EFFORT} -o "${work}/final.txt" - < "${work}/prompt.md" > "${work}/run.log" 2> "${work}/header.err"`
    : `codex exec -m ${CONFIG.CLI_MODEL} -c model_reasoning_effort=${CONFIG.CLI_EFFORT} -s workspace-write -o "${work}/final.txt" - < "${work}/prompt.md" > "${work}/run.log" 2> "${work}/header.err"`
  const launch = `mkdir -p "${work}" && rm -f "${work}/exit.code" && cd "${CONFIG.REPO}" && setsid nohup bash -c '${cmd}; echo "EXIT=$?" > "${work}/exit.code"' >/dev/null 2>&1 < /dev/null & echo LAUNCHED`
  const waitCmd = `timeout 580 tail -F "${work}/exit.code" 2>/dev/null | grep -m1 '^EXIT=' || cat "${work}/exit.code" 2>/dev/null || echo PENDING`
  return [
    hdr(stepId),
    'Mechanical wrapper for the CLI. You run provided commands and copy results. No initiative.',
    `STEP 1 — with the Write tool (NEVER via the shell: the prompt contains backticks and $() that would be interpreted), create the directory and write ${work}/prompt.md whose content is EXACTLY the delimited block below, without the delimiters, adding nothing, rephrasing nothing:`,
    '--- BEGIN prompt.md CONTENT ---',
    instruction,
    '--- END prompt.md CONTENT ---',
    'STEP 2 — launch the CLI detached via Bash, exactly:',
    launch,
    resumeSessionId
      ? `Resume by exact ID only (--last FORBIDDEN). If ${work}/header.err or ${work}/run.log contains "no rollout found for thread id", the session no longer exists: do NOT create a new session, return sessionFound=false and completed=false.`
      : 'New session.',
    'STEP 2b — WAIT: the CLI may work for over an hour. FORBIDDEN: Monitor, run_in_background, or ending your turn to wait (you would be killed before returning the result). Repeat this BLOCKING Bash call (tool timeout parameter = 600000) until its output contains an EXIT= line:',
    waitCmd,
    `Maximum 24 repetitions (≈ 4 h). If still PENDING after 24: run pkill -f "${work}" then return completed=false (fabricate no values).`,
    'STEP 3 — extract the EFFECTIVE values from the header, via Bash:',
    `grep -oP '^(model|reasoning effort|session id): .*' "${work}/header.err"`,
    `STEP 4 — read the CLI final message: ${work}/final.txt, and the exit code: ${work}/exit.code`,
    'RETURN — copy VERBATIM from real outputs, inventing and fixing nothing:',
    '- sessionId = the value after "session id:" in the header;',
    '- reportedModel / reportedEffort = the EFFECTIVE header values, never the requested ones;',
    `- completed = true only if ${work}/exit.code contains EXIT=0; sessionFound = false only on "no rollout found";`,
    `- finalOutput = the FULL VERBATIM content of ${work}/final.txt (data channel: no summarizing, truncating, reformatting);`,
    '- summary = 5 lines max for the human journal.',
    'Forbidden: modifying repo files, git add/commit/push, running anything beyond the commands above, relaunching the CLI on failure.',
  ].join('\n')
}
```

The driver then verifies PROOF of the effective model/effort and session:

```js
async function cli(stepId, phaseName, resumeSessionId, instruction) {
  const res = await agent(cliWrapperPrompt(stepId, resumeSessionId, instruction),
    { schema: CLI_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH })
  if (!res) pause('cli_wrapper_failed', { stepId }, stepId)
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(res.sessionId)) {
    pause('cli_session_id_invalid', { stepId, got: res.sessionId }, stepId)
  }
  if (resumeSessionId && !res.sessionFound) {
    pause('required_agent_session_unavailable', { stepId, sessionId: resumeSessionId,
      hint: 'Required CLI session is gone (pruned, other machine). Lost context is not reconstructible — human arbitration required.' }, stepId)
  }
  if (res.reportedModel !== CONFIG.CLI_MODEL || res.reportedEffort !== CONFIG.CLI_EFFORT) {
    pause('unexpected_effective_model', { stepId,
      expected: { model: CONFIG.CLI_MODEL, effort: CONFIG.CLI_EFFORT },
      got: { model: res.reportedModel, effort: res.reportedEffort } }, stepId)
  }
  if (resumeSessionId && res.sessionId !== resumeSessionId) {
    pause('cli_session_mismatch', { stepId, expected: resumeSessionId, got: res.sessionId }, stepId)
  }
  return res
}
```

Probe ALL of this on the real CLI version BEFORE launch: header format,
`--json` capabilities, resume flag inheritance, unknown-id behavior, and a
run longer than the Bash cap. The detached-launch pattern exists because the
un-probed >600 s path killed a real run in phase 1.

## 5. Findings pipeline (closed-mandate resolution)

One discovery review per phase (never re-run after corrections), then a
strictly narrowing pipeline. Each stage's mandate is CLOSED and the driver —
not the prompt — enforces containment.

```
discovery review (read-only, sandwiched, findings by schema)
  → fact-check by the implementer (same CLI session, analysis-only, sandwiched)
  → closed arbitration (fresh reviewer; only REQUIRED_FIX / DISMISSED / ESCALATE,
    only on the listed findingIds — driver pauses on out-of-mandate verdicts)
  → ESCALATE resolution via HUMAN_ARBITRATION table (driver applies the
    human verdict on resume; unresolved escalations pause)
  → bounded correction rounds (CLI applies ONLY arbitrated fixes; gate
    failures ADD to the fix list, never replace it)
  → closed verification (fresh reviewer; PASS / REOPEN / PAUSE, reopened
    findings must reference ledger findingIds; human DISMISSED wins)
```

Key containment snippets:

```js
// Driver-enforced closed mandate:
const foreign = arb.verdicts.filter(v => !findingIds.has(v.findingId))
if (foreign.length) pause('arbitration_out_of_mandate', { phase: p, foreign }, `${tag}.arbitration`)

// Human escalation channel (editable table, read only by the driver):
const escalations = arb.verdicts.filter(v => v.verdict === 'ESCALATE')
const unresolved = escalations.filter(v => !HUMAN_ARBITRATION[v.findingId])
if (unresolved.length) {
  pause('finding_requires_human_arbitration', { phase: p, escalations: unresolved,
    hint: 'HUMAN GATE: edit HUMAN_ARBITRATION at the top of the script — [findingId] = {verdict:"REQUIRED_FIX"|"DISMISSED", requiredOutcome?} — then Workflow({scriptPath, resumeFromRunId}). Read only by the driver: prior cache preserved.' })
}
```

The fact-check result travels through the CLI's `finalOutput` (verbatim data
channel), parsed by a separate mechanical agent — never through a summary.
Findings schema requires: exact location, violated contract, concrete
failure scenario, evidence, minimal required outcome. Admissibility is a
schema property, not a reviewer mood.

Three rules paid for by one real loop (P0 retro, 2026-07-25):

- **The ledger carries EVERY human decision, DISMISSED included.** A ledger
  carrying only the REQUIRED_FIX entries shows the verifier
  `humanArbitration: []` — which it reads as "no decision was ever made" —
  and the human-closed escalation gets reopened.
- **Apply the human-arbitration filter BEFORE honoring any verdict
  channel.** A PAUSE escape hatch checked first silently overrides a human
  DISMISSED and the pause loops forever:

```js
if (verif.verdict === 'PAUSE') {
  const stillOpen = verif.reopened.filter(r => {
    const d = HUMAN_ARBITRATION[r.findingId]
    return !(d && d.verdict === 'DISMISSED')
  })
  // PAUSE aimed only at human-closed findings: the question is answered —
  // treat as PASS. Any other PAUSE (including one with no reopened list,
  // which is a deliberate call for a human) pauses as before.
  if (!verif.reopened.length || stillOpen.length) {
    pause('compliance_requested_pause', { phase: p, verif, stillOpen }, `${tag}.verify.${round}`)
  }
}
```

- **Review scope ⊆ commit allowlist.** Findings on files the phase cannot
  stage (scratchpad scripts, other repos, already-executed probes) are
  operator notes, not phase blockers — say so in the review prompt, or
  arbitration burns agents dismissing them one by one (measured: 6 of 10
  findings in a doc-only slice).

## 6. Commits judged on observed state

Exit codes are LLM-reported; git state is not. The driver commits, then
judges success by observation — which also makes the commit idempotent
across resumes (a resume after a misreport does not re-commit).

```js
// Committed-check at every phase start (cold-restart robustness): a phase
// whose driver commit exists is NEVER replayed. Exact subject match in JS.
const marks = await exec(`p${p}.committed-check`, phaseName,
  ['git log --format="%H %P %s" -n 50'], `phase ${p} already committed? (cold resume)`)
const markLine = (marks[0].outputTail || '').split('\n').map(l => l.trim())
  .find(l => l.split(' ').slice(2).join(' ') === CONFIG.COMMIT_SUBJECT(p))
if (markLine) { /* record sha, skip phase */ }

// Driver-only commit with anti-pollution guard, single short-circuited command:
await exec(`p${p}.commit`, phaseName, [
  `bash -c 'git add -A -- . ${PROTECT_EXCLUDES} && ! git diff --cached --name-only | grep -E "${GUARD_PATTERN}" && git commit -m "${CONFIG.COMMIT_SUBJECT(p)}"'`,
], `driver commit phase ${p} (add + anti-pollution guard + commit)`)

// Success = observed state, not reported exit code:
const post = await fingerprint(`p${p}.post`, phaseName)
const committed = post.head !== base.head && post.statusSha256 === baseline.statusSha256
if (!committed) {
  pause('driver_commit_failed', {
    phase: p,
    headMoved: post.head !== base.head,
    worktreeClean: post.statusSha256 === baseline.statusSha256,
    protectedChanged: post.protectedSha256 !== baseline.protectedSha256,
    hint: post.head === base.head
      ? 'Commit did not happen (anti-pollution guard? nothing to commit?). Inspect the index, fix, bump the retryStep.'
      : post.protectedSha256 !== baseline.protectedSha256
        ? 'Commit done but a PROTECTED path changed since baseline — likely a user edit mid-run, not an agent residue. Verify, restore or accept, then bump the retryStep.'
        : 'Commit done but residue outside protected paths in the worktree (unstaged file? misplaced gate script?). Fix then bump the retryStep.',
  }, `p${p}.post`)
}
```

Empirically validated: saved a phase commit after the executor misreported
the same commit twice. The baseline itself is captured at preflight AFTER an
empty-index gate (a driver commit embarks the whole index, so the baseline
must reflect the remediated tree).

## 7. Preflight proves the gate green

A tool inventory (`x --version`) is not a preflight. Run the FULL
deterministic gate on the virgin tree before phase 1: a red gate here means
some phase will have to write outside its allowlist to validate itself — a
configuration dead-end that otherwise surfaces as an unfixable escalation
at commit time (measured: one implementation attempt burned, three human
round-trips).

```js
const g0 = await exec('preflight.gate', 'Preflight', CONFIG.GATE, 'gate green on the virgin tree?')
const g0fail = g0.filter(r => r.exitCode !== 0)
if (g0fail.length) {
  pause('gate_red_on_virgin_tree', { failures: g0fail,
    hint: 'HUMAN GATE: the gate fails BEFORE any phase — no slice can validate without out-of-mandate writes. Fix the tree (own commit), then bump RETRY_TOKENS["preflight.gate"].' },
    'preflight.gate')
}
```

In template.js this run is folded into the single preflight `probe()` agent
(index check + toolchain + gate + baseline fingerprint — one boot).
