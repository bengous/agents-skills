export const meta = {
  name: 'my-deterministic-driver',
  description: 'Deterministic driver for <task>: schema-constrained executors, block-sandwiched reviews, driver-only commits, file-based resume channel',
  whenToUse: 'Do NOT launch without human validation. Configuration lives in the CONFIG section; after a pause: edit the top-of-file constants (RETRY_TOKENS/HUMAN_ARBITRATION/MUTATION_ACK) then Workflow({scriptPath, resumeFromRunId}) — the args parameter is ignored on resume.',
  phases: [
    { title: 'Preflight', detail: 'index, toolchain, gate green on virgin tree, baseline — one agent' },
    { title: 'Phase 0', detail: '<stub>' },
    { title: 'Phase 1', detail: '<stub>' },
    { title: 'Final acceptance', detail: 'global validation + final audit' },
  ],
}

// ============================================================================
// MAPPING DECISIONS — document here every deviation between the source spec
// and this driver (paid once at first read, available at every reopen):
// which gates map to commands, which to fixed-path deliverable scripts,
// which to review judgments; which human gates are delegated to the
// orchestrator; probed facts about any external CLI (version, resume
// semantics per flag, header format).
//
// AGENT BUDGET (rule 7 — write the real numbers for YOUR driver before it
// may run; log() reports the actual count per phase):
//   Preflight: 1 (index + toolchain + gate + baseline, one probe)
//   Per phase, nominal: 6 (base 1, impl 1, val 1, review 1, block-end 1,
//     commit 1) — 4 on a review-policy 'none' phase.
//   Per extra validation attempt: +2 (impl, val).
//   Findings pipeline (full policy): see patterns.md §5.
//
// Core invariants of the architecture (see deterministic-driver skill):
// - Git is the only durable state; the prefix cache is disposable. Cold
//   restart is always possible: committed-check skips finished phases.
// - This script lives OUTSIDE the target repo (invoked by scriptPath).
// - Every pause carries a retryStep (the OBSERVATION to replay) and a
//   self-sufficient howToResume.
// - No conditional tool/skill branches in agent prompts; rules stated as
//   OPERATIONS (cp allowed / open forbidden), never intentions (read-only).
// - Commits are judged on observed state, never on reported exit codes.
// - ONE mechanical agent per contiguous mechanical sequence (rule 7):
//   probe() below returns command results AND a fingerprint in one boot.
// - Fingerprint sandwich per contiguous READ-ONLY BLOCK, chained — never
//   two probes back-to-back with nothing between them (rule 7).
// - A human decision is terminal: the findings ledger carries EVERY human
//   verdict (DISMISSED included), and no verdict channel bypasses the
//   human-arbitration filter (rule 13; code in patterns.md §5).
// ============================================================================

// --- RESUME CHANNEL: constants edited by the operator between resumes ------
// args is IGNORED on resume — this file is the only channel.
// Visibility classes (rule from patterns.md §1): RETRY_TOKENS is
// prompt-visible BY DESIGN (its invalidation mechanism). Every other resume
// constant is driver-only and must NEVER feed a value that appears in any
// prompt — widening one mid-run would invalidate the cache and regenerate
// the finding IDs the resume keys reference.

// Bump a step's token => that step and everything after re-run LIVE; the
// rest replays from cache. The pause names the exact stepId. When a pause
// carries retryStep: null (pure JS gate: arbitration, acks), bump nothing —
// the edited constant is the entire channel.
const RETRY_TOKENS = {}

// Human decisions on escalated findings: [findingId] = {verdict:
// 'REQUIRED_FIX'|'DISMISSED', requiredOutcome?}. Read only by the driver —
// and carried WHOLE (DISMISSED included) into every downstream ledger.
const HUMAN_ARBITRATION = {}

// Post-mutation acknowledgment: after a <watched_state>_mutation_suspected
// pause, resume requires an entry attesting the verification performed
// (what was compared, what was clean); the driver then re-baselines
// explicitly. [stepId] = 'free-text attestation'.
const MUTATION_ACK = {}

// ============================================================================
// CONFIG — everything project/machine specific. The body references ONLY
// CONFIG (plus the resume constants above).
// ============================================================================

const CONFIG = {
  NAME: 'my-deterministic-driver',
  REPO: '/abs/path/to/target-repo',
  BRANCH: 'feature/my-branch',
  SPEC: 'spec.md',                       // frozen contract, sha256-verified each phase
  PHASES: [0, 1],
  MAX_VALIDATION_ATTEMPTS: 3,
  MAX_COMPLIANCE_RETRIES: 1,

  // Agent tiers. MECH = the cheapest model that reliably runs listed
  // commands and copies output verbatim — NEVER the frontier model for
  // probes (rule 7: the boot overhead is identical, the work is mechanical).
  REVIEW: { model: 'opus', effort: 'xhigh' },   // reviews, arbitration
  MECH: { model: 'sonnet', effort: 'low' },     // mechanical probes/executors

  // External CLI implementer (optional — delete cli* if unused).
  CLI_MODEL: 'gpt-5.6-sol',
  CLI_EFFORT: 'high',
  CLI_WORK: '/tmp/my-driver-work',       // outside the repo: work files must not pollute the worktree baseline

  // Toolchain inventory (existence checks). The gate RUN below is what
  // actually proves the tree healthy — inventory alone is not a preflight.
  TOOLCHAIN: [
    'echo "replace with tool --version checks"',
  ],
  // Deterministic gate run at every validation AND at preflight on the
  // virgin tree (rule 12): a red gate before phase 1 means some phase would
  // have to write outside its allowlist to validate itself.
  GATE: [
    'echo "replace with fmt/lint/test/build commands"',
  ],
  // Per-phase extra gates (commands or fixed-path deliverable scripts).
  PHASE_EXTRA_GATES: { 0: [], 1: [] },

  // Per-phase review policy (rule 14 — scale ceremony to stakes):
  // 'full'  = review + findings pipeline (code slices),
  // 'light' = single review, findings are operator notes unless blocking,
  // 'none'  = no review (pure mechanical/doc phases where the gate is the
  //           whole contract).
  REVIEW_POLICY: { 0: 'full', 1: 'full' },

  // Protected paths: never staged, never modified by an agent. Protect the
  // FROZEN FILES the run owns, not shared parent directories other sessions
  // write to (skill rule 11: a concurrent legitimate write in a watched
  // directory pauses every phase).
  PROTECTED: ['plans/frozen-spec.md'],

  COMMIT_SUBJECT: p => `feat: complete phase ${p}`,
}

const PROTECT_EXCLUDES = CONFIG.PROTECTED.map(p => `":(exclude)${p}"`).join(' ')
const GUARD_PATTERN = `^(target/|${CONFIG.PROTECTED.map(p => p.replace(/\./g, '\\.')).join('|')})`

// --- Schemas ----------------------------------------------------------------
// Rule: every required field must feed a driver judgment. A field the driver
// never reads is the one an executor can invent for free — drop it.

// One schema for the merged mechanical agent: command results AND
// fingerprint in a single boot (rule 7). probe([]) is a bare fingerprint.
const PROBE_SCHEMA = {
  type: 'object',
  required: ['results', 'branch', 'head', 'specSha256', 'indexSha256', 'statusSha256', 'diffSha256', 'protectedSha256'],
  properties: {
    results: {
      type: 'array',
      items: {
        type: 'object',
        required: ['command', 'exitCode', 'outputTail'],
        properties: {
          command: { type: 'string', description: 'Command executed, VERBATIM' },
          exitCode: { type: 'integer' },
          outputTail: { type: 'string', description: 'Last 60 lines max of stdout+stderr, verbatim' },
        },
      },
    },
    branch: { type: 'string' },
    head: { type: 'string' },
    specSha256: { type: 'string' },
    indexSha256: { type: 'string' },
    statusSha256: { type: 'string' },
    diffSha256: { type: 'string' },
    protectedSha256: { type: 'string' },
  },
}

const CLI_SCHEMA = {
  type: 'object',
  required: ['sessionId', 'reportedModel', 'reportedEffort', 'completed', 'sessionFound', 'summary', 'finalOutput'],
  properties: {
    sessionId: { type: 'string', description: 'Exact CLI session ID (never --last)' },
    reportedModel: { type: 'string', description: 'Effective model reported by the CLI runtime' },
    reportedEffort: { type: 'string', description: 'Effective reasoning effort reported by the CLI runtime' },
    completed: { type: 'boolean' },
    sessionFound: { type: 'boolean', description: 'false if a resume by ID failed because the session no longer exists' },
    summary: { type: 'string', description: '5 lines max' },
    finalOutput: { type: 'string', description: 'CLI final message VERBATIM, not summarized, not truncated' },
  },
}

const FINDINGS_SCHEMA = {
  type: 'object',
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'file', 'summary', 'contractViolated', 'failureScenario', 'evidence', 'minimalRequiredOutcome'],
        properties: {
          id: { type: 'string' },
          file: { type: 'string' },
          line: { type: 'integer' },
          summary: { type: 'string' },
          contractViolated: { type: 'string' },
          failureScenario: { type: 'string' },
          evidence: { type: 'string' },
          minimalRequiredOutcome: { type: 'string' },
          outOfMandate: { type: 'boolean', description: 'true when the file cannot be staged by this phase (outside the commit allowlist) — operator note, not a phase blocker' },
        },
      },
    },
  },
}

// --- Pause / resume mechanics -------------------------------------------------

class Pause {
  constructor(reason, detail, retryStep) {
    this.reason = reason
    this.detail = detail
    this.retryStep = retryStep
  }
}
// retryStep designates the OBSERVATION to replay (fingerprint, validation),
// not the cached work — without it, the failure result would replay from
// cache on resume and the pause would loop forever. null is legitimate ONLY
// for pure JS gates (arbitration, acknowledgment constants) where the
// edited constant is the whole channel.
function pause(reason, detail, retryStep) {
  throw new Pause(reason, detail, retryStep)
}

function rt(stepId) { return RETRY_TOKENS[stepId] || 0 }
function hdr(stepId) { return `[${CONFIG.NAME} step:${stepId} retry:${rt(stepId)}]` }

// Agent economy instrumentation (rule 7): a cost nobody sees is a cost
// nobody debates. Incremented by every agent call; logged per phase.
let AGENT_COUNT = 0

// --- Merged mechanical probe (exec + fingerprint, one boot) ---------------------

const MECH_RULES = [
  'Strict rules:',
  '- You are a mechanical executor for the driver. NO interpretation, NO initiative.',
  '- Run nothing beyond the listed commands. Fix nothing. Retry nothing.',
  '- A failing command: record its result and move to the next.',
  '- Modify no files. Forbidden: git add/commit/push/reset/stash, rm, writes outside the repo.',
  '- Copy values VERBATIM from real outputs. No invented values.',
  '- Report ONE result per listed command, same order, command copied verbatim.',
].join('\n')

// ONE agent = listed commands (in order, verbatim report) THEN the
// fingerprint block, LAST. probe([]) = bare fingerprint. This is the merged
// shape rule 7 mandates; patterns.md §2/§3 document the split primitives.
async function probe(stepId, phaseName, commands = [], context = 'driver probe') {
  AGENT_COUNT++
  const prompt = [
    hdr(stepId),
    `Context: ${context}`,
    commands.length
      ? `From ${CONFIG.REPO}, run via Bash, in order, exactly these ${commands.length} commands (one results entry per command):`
      : `From ${CONFIG.REPO}: no listed commands — return results as an empty array.`,
    ...commands.map(c => `- ${c}`),
    'THEN — always LAST, after every listed command has run — execute the fingerprint block and copy values verbatim:',
    '- branch : git rev-parse --abbrev-ref HEAD',
    '- head : git rev-parse HEAD',
    `- specSha256 : sha256sum ${CONFIG.SPEC} (first field)`,
    '- indexSha256 : git diff --cached | sha256sum (first field)',
    '- statusSha256 : git status --porcelain=v2 --untracked-files=all | LC_ALL=C sort | sha256sum (first field)',
    '- diffSha256 : git diff | sha256sum (first field)',
    `- protectedSha256 : find ${CONFIG.PROTECTED.join(' ')} -type f -print0 2>/dev/null | LC_ALL=C sort -z | xargs -0 -r sha256sum | sha256sum (first field)`,
    MECH_RULES,
    'Timeouts: pass timeout=600000 to the Bash tool for each command. If a command hits this timeout and is moved to the background by the harness, do NOT wait for it and do not end your turn: report exitCode=-1 and outputTail="TIMEOUT_BACKGROUNDED" for it, then continue the list.',
    'For each listed command return: command (verbatim), exitCode, outputTail (last 60 lines max).',
  ].join('\n')
  const out = await agent(prompt, { schema: PROBE_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH })
  if (!out) pause('probe_agent_failed', { stepId }, stepId)
  if (out.branch !== CONFIG.BRANCH) pause('wrong_branch', { expected: CONFIG.BRANCH, got: out.branch }, stepId)
  // Anti-empty-gate: 1:1 command/result correspondence, verbatim modulo the
  // `bash -c '...'` envelope (executors strip it — observed repeatedly).
  if (out.results.length !== commands.length) {
    pause('probe_results_mismatch', { stepId, expected: commands.length, got: out.results.length }, stepId)
  }
  const normCmd = c => {
    const t = c.trim()
    const m = t.match(/^bash -c '([\s\S]*)'$/)
    return (m ? m[1] : t).trim()
  }
  const mismatched = out.results
    .map((r, i) => (normCmd(r.command) === normCmd(commands[i]) ? null : { index: i, expected: commands[i], got: r.command }))
    .filter(Boolean)
  if (mismatched.length) pause('probe_results_mismatch', { stepId, mismatched }, stepId)
  return out
}

// afterStepId = the step to re-probe/re-run after human remediation.
// Compares the five content fields of any two probe results.
function assertUnchanged(before, after, reason, phaseNum, afterStepId) {
  const same =
    before.head === after.head &&
    before.indexSha256 === after.indexSha256 &&
    before.statusSha256 === after.statusSha256 &&
    before.diffSha256 === after.diffSha256 &&
    before.protectedSha256 === after.protectedSha256
  if (!same) pause(reason, { phase: phaseNum, before, after }, afterStepId)
}

// --- External CLI wrapper (optional) --------------------------------------------
// Detached launch + blocking waits: a schema-enforced agent can never end its
// turn to wait, and one Bash call caps at 600 s while the CLI may run hours.
// Probe your CLI BEFORE launch: header format, resume flag inheritance,
// unknown-id behavior, a run longer than the Bash cap.

function cliWrapperPrompt(stepId, resumeSessionId, instruction) {
  const work = `${CONFIG.CLI_WORK}/${stepId.replace(/[^A-Za-z0-9._-]/g, '_')}`
  const cmd = resumeSessionId
    // Resume: effort re-passed explicitly (codex `exec resume` silently drops it).
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

async function cli(stepId, phaseName, resumeSessionId, instruction) {
  AGENT_COUNT++
  const res = await agent(cliWrapperPrompt(stepId, resumeSessionId, instruction), {
    schema: CLI_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH,
  })
  if (!res) pause('cli_wrapper_failed', { stepId }, stepId)
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(res.sessionId)) {
    pause('cli_session_id_invalid', { stepId, got: res.sessionId }, stepId)
  }
  if (resumeSessionId && !res.sessionFound) {
    pause('required_agent_session_unavailable', {
      stepId, sessionId: resumeSessionId,
      hint: 'Required CLI session is gone (pruned, other machine). Lost context is not reconstructible — human arbitration required.',
    }, stepId)
  }
  if (res.reportedModel !== CONFIG.CLI_MODEL || res.reportedEffort !== CONFIG.CLI_EFFORT) {
    pause('unexpected_effective_model', {
      stepId, expected: { model: CONFIG.CLI_MODEL, effort: CONFIG.CLI_EFFORT },
      got: { model: res.reportedModel, effort: res.reportedEffort },
    }, stepId)
  }
  if (resumeSessionId && res.sessionId !== resumeSessionId) {
    pause('cli_session_mismatch', { stepId, expected: resumeSessionId, got: res.sessionId }, stepId)
  }
  return res
}

// --- Phase cycle ------------------------------------------------------------------

function implInstruction(p, attempt, lastValidation) {
  const lines = [
    `You are implementing Phase ${p} of <task> (${CONFIG.REPO}).`,
    `Contract: ${CONFIG.SPEC}. Read the file; its content is frozen (the driver verifies its sha256).`,
    'Non-negotiable rules:',
    '- No commit, no push, no mutating git command.',
    `- No writes outside ${CONFIG.REPO}. No writes in: ${CONFIG.PROTECTED.join(', ')}.`,
    '- No unrequested refactor, no speculative abstraction or dependency.',
    // STUB: state every environment rule as OPERATIONS (cp allowed, open
    // forbidden), never intentions ("read-only") — skill rule 3.
    `The gate to turn green: ${JSON.stringify([...(CONFIG.PHASE_EXTRA_GATES[p] || []), ...CONFIG.GATE])}.`,
  ]
  if (attempt > 1 && lastValidation) {
    lines.push(
      `Attempt ${attempt}/${CONFIG.MAX_VALIDATION_ATTEMPTS}. The gate failed. Failures (command, exitCode, output tail):`,
      JSON.stringify(lastValidation.failures, null, 2),
      'Fix only what turns the gate green. No gaming of the checks.',
    )
  }
  return lines.join('\n')
}

function discoveryPrompt(stepId, p, reviewBaseSha) {
  return [
    hdr(stepId),
    `You are the SINGLE discovery review of Phase ${p} of <task> (${CONFIG.REPO}). You are STRICTLY read-only: no file writes, no mutating git command, no subagents.`,
    `Target: the diff \`git diff ${reviewBaseSha}\` (worktree changes included — the driver has not committed the phase yet).`,
    `Contract: ${CONFIG.SPEC}, Phase ${p} plus associated contract sections.`,
    // Mandatory methodology — no "if available" branch (skill rule 2):
    'MANDATORILY invoke the code-review skill via the Skill tool, at xhigh level. Sole exception: technical invocation failure — then perform an equivalent review by reading the diff and files, and report that failure in the summary of a finding.',
    'An admissible finding MUST name: exact location, violated contract/invariant, concrete failure scenario, evidence (code path or reproducer), minimal required outcome.',
    // Review scope ⊆ commit allowlist (skill rule 14): unfixable findings
    // must not enter the blocking pipeline.
    'Your scope is what this phase can COMMIT. A finding on a file the phase cannot stage (outside the repo, scratchpad artifacts, already-executed probes) is an operator note: set outOfMandate=true on it — it will be reported, not arbitrated.',
    'A style preference, hypothetical improvement, or preexisting issue is not automatically a phase blocker — flag it as such in summary if you report it.',
    'Return the (possibly empty) list via the schema. Modify NOTHING.',
  ].join('\n')
}

async function runPhase(p, phaseName, baseline, base) {
  const agentsAtStart = AGENT_COUNT

  // Implementation / validation — bounded attempts, one CLI session per phase.
  let sessionId = null
  let validation = null
  let val = null
  for (let attempt = 1; attempt <= CONFIG.MAX_VALIDATION_ATTEMPTS; attempt++) {
    const res = await cli(`p${p}.impl.${attempt}`, phaseName, attempt === 1 ? null : sessionId, implInstruction(p, attempt, validation))
    sessionId = res.sessionId

    // ONE agent: post-impl mutation checks + deterministic gate (rule 7).
    // The fingerprint runs LAST inside the agent, after the gate commands.
    const gateCommands = [...(CONFIG.PHASE_EXTRA_GATES[p] || []), ...CONFIG.GATE]
    val = await probe(`p${p}.val.${attempt}`, phaseName, gateCommands, `phase ${p}: mutation checks + deterministic gate, attempt ${attempt}`)
    if (val.head !== base.head) pause('agent_commit_detected', { phase: p, attempt }, `p${p}.val.${attempt}`)
    if (val.indexSha256 !== base.indexSha256) pause('agent_staged_changes', { phase: p, attempt }, `p${p}.val.${attempt}`)
    if (val.protectedSha256 !== base.protectedSha256) {
      pause('protected_paths_modified', { phase: p, attempt }, `p${p}.val.${attempt}`)
    }

    const failures = val.results.filter(r => r.exitCode !== 0)
    validation = { ok: failures.length === 0, failures, results: val.results }
    log(`${phaseName}: validation attempt ${attempt} → ${validation.ok ? 'GREEN' : `${validation.failures.length} failure(s)`}`)
    if (validation.ok) break
    if (attempt === CONFIG.MAX_VALIDATION_ATTEMPTS) {
      pause('phase_validation_failed', { phase: p, failures: validation.failures }, `p${p}.val.${attempt}`)
    }
  }

  // Review — policy-gated (rule 14). The read-only block (review, and in a
  // full pipeline fact-check + arbitration too) shares ONE sandwich: its
  // `before` is the last validation probe, already in hand; its `after` is
  // the single bare probe below. Never probe between two read-only agents.
  const policy = CONFIG.REVIEW_POLICY[p] || 'full'
  if (policy !== 'none') {
    AGENT_COUNT++
    const review = await agent(discoveryPrompt(`p${p}.review`, p, base.head), {
      schema: FINDINGS_SCHEMA, label: `p${p}.review`, phase: phaseName, ...CONFIG.REVIEW,
    })
    if (!review) pause('discovery_review_failed', { phase: p }, `p${p}.review`)
    const blockers = review.findings.filter(f => !f.outOfMandate)
    const notes = review.findings.filter(f => f.outOfMandate)
    log(`${phaseName}: discovery review → ${blockers.length} blocker(s), ${notes.length} operator note(s)`)

    if (blockers.length > 0) {
      // STUB: findings resolution pipeline ('full' policy) — fact-check
      // (read-only, INSIDE this same sandwich) → closed arbitration
      // (REQUIRED_FIX/DISMISSED/ESCALATE on listed findingIds only; driver
      // pauses on out-of-mandate verdicts) → ESCALATE resolved via
      // HUMAN_ARBITRATION or pause → bounded correction rounds → closed
      // verification. TWO NON-NEGOTIABLES (rule 13): the ledger passed
      // downstream carries EVERY human verdict, DISMISSED included; and the
      // human-arbitration filter runs BEFORE any verdict channel (a PAUSE
      // aimed only at human-closed findings is a PASS). Full code:
      // patterns.md §5.
      pause('findings_pipeline_not_implemented', { phase: p, findings: blockers }, `p${p}.review`)
    }

    // Close the read-only block: one bare probe, chained from `val`.
    const blockEnd = await probe(`p${p}.readonly.end`, phaseName)
    assertUnchanged(val, blockEnd, 'readonly_block_mutated_repo', p, `p${p}.readonly.end`)
  }

  // Driver-only commit + post-state fingerprint: ONE agent (rule 7). The
  // fingerprint runs after the commit command, so it IS the post state.
  const commitCmd = `bash -c 'git add -A -- . ${PROTECT_EXCLUDES} && ! git diff --cached --name-only | grep -E "${GUARD_PATTERN}" && git commit -m "${CONFIG.COMMIT_SUBJECT(p)}"'`
  const post = await probe(`p${p}.commit`, phaseName, [commitCmd], `driver commit phase ${p} (add + anti-pollution guard + commit + post fingerprint)`)

  // Commit success judged on OBSERVED STATE, not reported exit codes
  // (idempotence: a resume after a misreport does not re-commit).
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
          ? 'Commit done but a PROTECTED path changed since baseline — likely a user edit mid-run. Verify, restore or accept, then bump the retryStep.'
          : 'Commit done but residue outside protected paths (unstaged file? misplaced script?). Fix then bump the retryStep.',
    }, `p${p}.commit`)
  }
  log(`${phaseName}: ${AGENT_COUNT - agentsAtStart} agents (budget: see mapping-decisions block)`)
  return { commitSha: post.head, sessionId }
}

// --- Main -------------------------------------------------------------------------

try {
  phase('Preflight')

  // ONE preflight agent (rule 7): index inventory + toolchain + gate on the
  // virgin tree (rule 12) + baseline fingerprint. The fingerprint runs LAST,
  // so the baseline reflects the tree the commands observed.
  const PRE_COMMANDS = ['git diff --cached --name-only', ...CONFIG.TOOLCHAIN, ...CONFIG.GATE]
  const pf = await probe('preflight', 'Preflight', PRE_COMMANDS, 'preflight: index empty + toolchain + gate green on virgin tree + baseline')

  // Empty index REQUIRED (a driver commit embarks the whole index, and the
  // baseline must reflect the remediated tree).
  if ((pf.results[0].outputTail || '').trim() !== '') {
    pause('index_not_clean', {
      staged: pf.results[0].outputTail,
      hint: 'HUMAN GATE: commit or unstage these user changes, then resume with a bump of RETRY_TOKENS["preflight"] — the baseline is captured by the same probe.',
    }, 'preflight')
  }
  const toolFailures = pf.results.slice(1, 1 + CONFIG.TOOLCHAIN.length).filter(r => r.exitCode !== 0)
  if (toolFailures.length) {
    pause('toolchain_missing', {
      failures: toolFailures,
      hint: 'HUMAN GATE: provision the missing tools (the driver installs NOTHING), then bump RETRY_TOKENS["preflight"].',
    }, 'preflight')
  }
  // Gate GREEN on the virgin tree (rule 12): a red gate here means some
  // phase would have to write outside its allowlist to validate itself.
  const gateFailures = pf.results.slice(1 + CONFIG.TOOLCHAIN.length).filter(r => r.exitCode !== 0)
  if (gateFailures.length) {
    pause('gate_red_on_virgin_tree', {
      failures: gateFailures,
      hint: 'HUMAN GATE: the deterministic gate fails BEFORE any phase. Fix the tree in its own commit (never inside a slice), then bump RETRY_TOKENS["preflight"].',
    }, 'preflight')
  }
  const baseline = pf
  log(`Preflight OK (1 agent) — base: ${baseline.head}, gate green on virgin tree, baseline frozen`)

  // Phases -----------------------------------------------------------------------
  const committed = {}
  const sessions = {}
  for (const p of CONFIG.PHASES) {
    const phaseName = `Phase ${p}`
    phase(phaseName)

    // ONE agent: committed-check + phase base fingerprint (rule 7). Durable
    // state lives in git — a phase whose driver commit exists is NEVER
    // replayed. Exact subject match in JS.
    const base = await probe(`p${p}.base`, phaseName, [
      'git log --format="%H %P %s" -n 50',
    ], `phase ${p} already committed? (cold resume) + phase base fingerprint`)
    if (base.specSha256 !== baseline.specSha256) {
      pause('spec_changed_mid_run', { phase: p, expected: baseline.specSha256, got: base.specSha256 }, `p${p}.base`)
    }
    const markLine = (base.results[0].outputTail || '')
      .split('\n')
      .map(l => l.trim())
      .find(l => l.split(' ').slice(2).join(' ') === CONFIG.COMMIT_SUBJECT(p))
    if (markLine) {
      committed[p] = markLine.split(' ')[0]
      sessions[p] = null // CLI session from the old run: resume not guaranteed
      log(`${phaseName} already committed (${committed[p]}) — skipped`)
      continue
    }

    const r = await runPhase(p, phaseName, baseline, base)
    committed[p] = r.commitSha
    sessions[p] = r.sessionId
    log(`${phaseName} committed: ${r.commitSha}`)
  }

  // Final acceptance ---------------------------------------------------------------
  phase('Final acceptance')
  // ONE agent: global gate re-run + final fingerprint (rule 7). STUB: add a
  // final audit (fresh REVIEW agent, sandwiched with this probe as before)
  // if the task warrants it — see patterns.md.
  const done = await probe('final.done', 'Final acceptance', CONFIG.GATE, 'global gate re-run + final state')
  const finalFailures = done.results.filter(r => r.exitCode !== 0)
  if (finalFailures.length) {
    pause('final_gate_failed', { failures: finalFailures }, 'final.done')
  }
  if (done.statusSha256 !== baseline.statusSha256) {
    pause('worktree_not_clean_at_end', { hint: 'Uncommitted residue outside protected paths.' }, 'final.done')
  }

  log(`ready_for_human_validation — nothing pushed, human product validation required (${AGENT_COUNT} agents total)`)
  return {
    status: 'ready_for_human_validation',
    baseHead: baseline.head,
    finalHead: done.head,
    phases: committed,
    cliSessions: sessions,
    agentCount: AGENT_COUNT,
  }
} catch (e) {
  if (e instanceof Pause) {
    log(`PAUSE: ${e.reason}${e.retryStep ? ` (retryStep: ${e.retryStep})` : ''}`)
    return {
      status: 'paused',
      reason: e.reason,
      detail: e.detail,
      retryStep: e.retryStep || null,
      howToResume: e.retryStep
        ? `1) Treat the cause. 2) Edit this script: RETRY_TOKENS["${e.retryStep}"] = <current + 1>. 3) Workflow({scriptPath, resumeFromRunId}). NEVER resume without the bump: the cached failure would replay and the pause would loop. The args parameter is ignored on resume — the file is the only channel.`
        : '1) Treat the cause. 2) Edit the constant named in the hint (HUMAN_ARBITRATION / MUTATION_ACK / ...) at the top of this script — retryStep is null: this is a pure JS gate, there is no observation to replay, bump nothing. 3) Workflow({scriptPath, resumeFromRunId}). These constants are read only by the driver: the prior cache is preserved.',
    }
  }
  throw e
}
