export const meta = {
  name: 'my-deterministic-driver',
  description: 'Deterministic driver for <task>: schema-constrained executors, fingerprint-sandwiched reviews, driver-only commits, file-based resume channel',
  whenToUse: 'Do NOT launch without human validation. Configuration lives in the CONFIG section; after a pause: edit the top-of-file constants (RETRY_TOKENS/HUMAN_ARBITRATION/MUTATION_ACK) then Workflow({scriptPath, resumeFromRunId}) — the args parameter is ignored on resume.',
  phases: [
    { title: 'Preflight', detail: 'repo invariants, toolchain, baseline' },
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
// Core invariants of the architecture (see deterministic-driver skill):
// - Git is the only durable state; the prefix cache is disposable. Cold
//   restart is always possible: committed-check skips finished phases.
// - This script lives OUTSIDE the target repo (invoked by scriptPath).
// - Every pause carries a retryStep (the OBSERVATION to replay) and a
//   self-sufficient howToResume.
// - No conditional tool/skill branches in agent prompts; rules stated as
//   OPERATIONS (cp allowed / open forbidden), never intentions (read-only).
// - Commits are judged on observed state, never on reported exit codes.
// ============================================================================

// --- RESUME CHANNEL: constants edited by the operator between resumes ------
// args is IGNORED on resume — this file is the only channel.

// Bump a step's token => that step and everything after re-run LIVE; the
// rest replays from cache. The pause names the exact stepId.
const RETRY_TOKENS = {}

// Human decisions on escalated findings: [findingId] = {verdict:
// 'REQUIRED_FIX'|'DISMISSED', requiredOutcome?}. Read only by the driver.
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

  // Agent tiers.
  REVIEW: { model: 'opus', effort: 'xhigh' },  // reviews, arbitration
  MECH: { model: 'opus', effort: 'low' },      // mechanical executors, probes, wrappers

  // External CLI implementer (optional — delete cli* if unused).
  CLI_MODEL: 'gpt-5.6-sol',
  CLI_EFFORT: 'high',
  CLI_WORK: '/tmp/my-driver-work',       // outside the repo: work files must not pollute the worktree baseline

  // Deterministic gate run at every validation.
  GATE: [
    'echo "replace with fmt/lint/test/build commands"',
  ],
  // Per-phase extra gates (commands or fixed-path deliverable scripts).
  PHASE_EXTRA_GATES: { 0: [], 1: [] },

  // Protected paths: never staged, never modified by an agent.
  PROTECTED: ['plans', '.claude', '.agents'],

  COMMIT_SUBJECT: p => `feat: complete phase ${p}`,
}

const PROTECT_EXCLUDES = CONFIG.PROTECTED.map(p => `":(exclude)${p}"`).join(' ')
const GUARD_PATTERN = `^(target/|${CONFIG.PROTECTED.map(p => p.replace(/\./g, '\\.') + '/').join('|')})`

// --- Schemas ----------------------------------------------------------------
// Rule: every required field must feed a driver judgment. A field the driver
// never reads is the one an executor can invent for free — drop it.

const EXEC_SCHEMA = {
  type: 'object',
  required: ['results'],
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
  },
}

const FP_SCHEMA = {
  type: 'object',
  required: ['branch', 'head', 'specSha256', 'indexSha256', 'statusSha256', 'diffSha256', 'protectedSha256'],
  properties: {
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
// retryStep MANDATORY: without it, the failure result would replay from
// cache on resume and the pause would loop forever. retryStep designates the
// OBSERVATION to replay (fingerprint, validation), not the cached work.
function pause(reason, detail, retryStep) {
  throw new Pause(reason, detail, retryStep)
}

function rt(stepId) { return RETRY_TOKENS[stepId] || 0 }
function hdr(stepId) { return `[${CONFIG.NAME} step:${stepId} retry:${rt(stepId)}]` }

// --- Mechanical executor (anti-empty-gate) -------------------------------------

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
    'Timeouts: pass timeout=600000 to the Bash tool for each command. If a command hits this timeout and is moved to the background by the harness, do NOT wait for it and do not end your turn: report exitCode=-1 and outputTail="TIMEOUT_BACKGROUNDED" for it, then continue the list.',
    'For each command return: command (verbatim), exitCode, outputTail (last 60 lines max).',
  ].join('\n')
  const out = await agent(prompt, { schema: EXEC_SCHEMA, label: stepId, phase: phaseName, ...CONFIG.MECH })
  if (!out) pause('executor_agent_failed', { stepId }, stepId)
  if (out.results.length !== commands.length) {
    pause('executor_results_mismatch', { stepId, expected: commands.length, got: out.results.length }, stepId)
  }
  // Comparison modulo the `bash -c '...'` envelope: executors strip it.
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

// --- Fingerprint sandwich (read-only enforcement) -------------------------------

async function fingerprint(stepId, phaseName) {
  const prompt = [
    hdr(stepId),
    `Read-only probe for the driver. From ${CONFIG.REPO}, run exactly these commands and copy values verbatim:`,
    '- branch : git rev-parse --abbrev-ref HEAD',
    '- head : git rev-parse HEAD',
    `- specSha256 : sha256sum ${CONFIG.SPEC} (first field)`,
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

// --- Deterministic validation ---------------------------------------------------

async function validatePhase(p, phaseName, stepId) {
  const extras = CONFIG.PHASE_EXTRA_GATES[p] || []
  const commands = [...extras, ...CONFIG.GATE]
  const results = await exec(stepId, phaseName, commands, `deterministic gate phase ${p}`)
  // STUB: add watched-state windows here if the task touches live external
  // state (detection check at BOTH ends of the window + fail-closed
  // before/after hash pair; see the deterministic-driver skill, rule 4).
  const failures = results.filter(r => r.exitCode !== 0)
  return { ok: failures.length === 0, failures, results }
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
    'A style preference, hypothetical improvement, or preexisting issue is not automatically a phase blocker — flag it as such in summary if you report it.',
    'Return the (possibly empty) list via the schema. Modify NOTHING.',
  ].join('\n')
}

async function runPhase(p, phaseName, baseline) {
  const base = await fingerprint(`p${p}.base`, phaseName)
  if (base.specSha256 !== baseline.specSha256) {
    pause('spec_changed_mid_run', { phase: p, expected: baseline.specSha256, got: base.specSha256 }, `p${p}.base`)
  }

  // Implementation / validation — bounded attempts, one CLI session per phase.
  let sessionId = null
  let validation = null
  for (let attempt = 1; attempt <= CONFIG.MAX_VALIDATION_ATTEMPTS; attempt++) {
    const res = await cli(`p${p}.impl.${attempt}`, phaseName, attempt === 1 ? null : sessionId, implInstruction(p, attempt, validation))
    sessionId = res.sessionId

    const fpNow = await fingerprint(`p${p}.impl.${attempt}.fp`, phaseName)
    if (fpNow.head !== base.head) pause('agent_commit_detected', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    if (fpNow.indexSha256 !== base.indexSha256) pause('agent_staged_changes', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    if (fpNow.protectedSha256 !== base.protectedSha256) {
      pause('protected_paths_modified', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    }

    validation = await validatePhase(p, phaseName, `p${p}.val.${attempt}`)
    log(`${phaseName}: validation attempt ${attempt} → ${validation.ok ? 'GREEN' : `${validation.failures.length} failure(s)`}`)
    if (validation.ok) break
    if (attempt === CONFIG.MAX_VALIDATION_ATTEMPTS) {
      pause('phase_validation_failed', { phase: p, failures: validation.failures }, `p${p}.val.${attempt}`)
    }
  }

  // Single discovery review — never re-run after corrections.
  const rBefore = await fingerprint(`p${p}.review.before`, phaseName)
  const review = await agent(discoveryPrompt(`p${p}.review`, p, base.head), {
    schema: FINDINGS_SCHEMA, label: `p${p}.review`, phase: phaseName, ...CONFIG.REVIEW,
  })
  if (!review) pause('discovery_review_failed', { phase: p }, `p${p}.review`)
  const rAfter = await fingerprint(`p${p}.review.after`, phaseName)
  assertUnchanged(rBefore, rAfter, 'discovery_review_mutated_repo', p, `p${p}.review`)
  log(`${phaseName}: discovery review → ${review.findings.length} finding(s)`)

  if (review.findings.length > 0) {
    // STUB: findings resolution pipeline — fact-check by the implementer
    // (same CLI session, analysis-only, fingerprint-sandwiched) → closed
    // arbitration (fresh REVIEW agent; REQUIRED_FIX/DISMISSED/ESCALATE on
    // listed findingIds only, driver pauses on out-of-mandate verdicts) →
    // ESCALATE resolved via HUMAN_ARBITRATION or pause → bounded correction
    // rounds (gate failures ADD to fixes) → closed verification
    // (PASS/REOPEN/PAUSE). Full code: patterns.md §5.
    pause('findings_pipeline_not_implemented', { phase: p, findings: review.findings }, `p${p}.review`)
  }

  // Driver-only commit; anti-pollution guard in one short-circuited command.
  await exec(`p${p}.commit`, phaseName, [
    `bash -c 'git add -A -- . ${PROTECT_EXCLUDES} && ! git diff --cached --name-only | grep -E "${GUARD_PATTERN}" && git commit -m "${CONFIG.COMMIT_SUBJECT(p)}"'`,
  ], `driver commit phase ${p} (add + anti-pollution guard + commit)`)

  // Commit success judged on OBSERVED STATE, not reported exit codes
  // (idempotence: a resume after a misreport does not re-commit).
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
          ? 'Commit done but a PROTECTED path changed since baseline — likely a user edit mid-run. Verify, restore or accept, then bump the retryStep.'
          : 'Commit done but residue outside protected paths (unstaged file? misplaced script?). Fix then bump the retryStep.',
    }, `p${p}.post`)
  }
  return { commitSha: post.head, sessionId }
}

// --- Main -------------------------------------------------------------------------

try {
  phase('Preflight')

  // Empty index REQUIRED BEFORE baseline capture: a driver commit embarks
  // the whole index, and the baseline must reflect the remediated tree.
  const idx = await exec('preflight.index', 'Preflight', ['git diff --cached --name-only'], 'index inventory')
  if ((idx[0].outputTail || '').trim() !== '') {
    pause('index_not_clean', {
      staged: idx[0].outputTail,
      hint: 'HUMAN GATE: commit or unstage these user changes, then resume with a bump of RETRY_TOKENS["preflight.index"] — the baseline is captured after.',
    }, 'preflight.index')
  }

  // Frozen run baseline: rewrite base head, reference status, spec sha,
  // protected content.
  const baseline = await fingerprint('preflight.fp', 'Preflight')

  // STUB: toolchain checks, backup verification, environment probes — every
  // externally-provisioned prerequisite gets a HUMAN GATE pause with a hint
  // naming the exact constant/step to edit. The driver installs NOTHING.

  log(`Preflight OK — base: ${baseline.head}, worktree baseline frozen`)

  // Phases -----------------------------------------------------------------------
  const committed = {}
  const sessions = {}
  for (const p of CONFIG.PHASES) {
    const phaseName = `Phase ${p}`
    phase(phaseName)

    // Cold-restart robustness: durable state lives in git — a phase whose
    // driver commit exists is NEVER replayed. Exact subject match in JS.
    const marks = await exec(`p${p}.committed-check`, phaseName, [
      'git log --format="%H %P %s" -n 50',
    ], `phase ${p} already committed? (cold resume)`)
    const markLine = (marks[0].outputTail || '')
      .split('\n')
      .map(l => l.trim())
      .find(l => l.split(' ').slice(2).join(' ') === CONFIG.COMMIT_SUBJECT(p))
    if (markLine) {
      committed[p] = markLine.split(' ')[0]
      sessions[p] = null // CLI session from the old run: resume not guaranteed
      log(`${phaseName} already committed (${committed[p]}) — skipped`)
      continue
    }

    const r = await runPhase(p, phaseName, baseline)
    committed[p] = r.commitSha
    sessions[p] = r.sessionId
    log(`${phaseName} committed: ${r.commitSha}`)
  }

  // Final acceptance ---------------------------------------------------------------
  phase('Final acceptance')
  // STUB: global gate re-run, final audit (fresh REVIEW agent, sandwiched),
  // optional final fix commit — see patterns.md.
  const done = await fingerprint('final.done', 'Final acceptance')
  if (done.statusSha256 !== baseline.statusSha256) {
    pause('worktree_not_clean_at_end', { hint: 'Uncommitted residue outside protected paths.' }, 'final.done')
  }

  log('ready_for_human_validation — nothing pushed, human product validation required')
  return {
    status: 'ready_for_human_validation',
    baseHead: baseline.head,
    finalHead: done.head,
    phases: committed,
    cliSessions: sessions,
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
        : '1) Treat the cause. 2) Edit the constant named in the hint (HUMAN_ARBITRATION / MUTATION_ACK / ...) at the top of this script. 3) Workflow({scriptPath, resumeFromRunId}). These constants are read only by the driver: the prior cache is preserved.',
    }
  }
  throw e
}
