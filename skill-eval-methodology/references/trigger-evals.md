# Trigger Evals

A trigger eval measures one thing: does the skill's `description` make the model
load the skill for the requests it is meant for, and only for those. It does not
measure what the skill produces once loaded; that is a capability eval.

## Contents

- [What the harness measures](#what-the-harness-measures)
- [Design the query set](#design-the-query-set)
- [Staged protocol](#staged-protocol)
- [Commands](#commands)
- [Pitfalls](#pitfalls)
- [Cost](#cost)
- [Worked example: state-audit](#worked-example-state-audit)

## What the harness measures

`skill-creator`'s `scripts/run_eval.py` writes the description as a command file
under `<project>/.claude/commands/<name>-skill-<uuid>.md`, runs `claude -p
"<query>"` with `--output-format stream-json --include-partial-messages`, and
stops at the model's **first tool call**. It counts a trigger when that call is
`Skill` or `Read` naming the uuid file, and a miss for anything else: another
skill, a `Bash`, a `Glob`, plain text, or the timeout.

So the score is "the skill is the model's first move", from an empty project.
In a real repository the model may explore files first and load the skill later;
the harness counts that as a miss. State this limit with every result.

## Design the query set

About 20 queries, written as a real user would: context, file names, the
symptom, sometimes French, sometimes lowercase and terse.

- **Should trigger (about 8):** cover each trigger condition of the description
  at least twice, and note which condition each query exercises.
- **Should not trigger (about 12):** near misses only. Each shares vocabulary
  with the skill and belongs elsewhere; note where (another skill, plain
  debugging, a question). "Write a fibonacci function" tests nothing.
- Flag the one query you are least sure of, and ask the owner whether it should
  trigger before running. A vocabulary trap is the best candidate: the skill's
  own terms in a request it should not take.

Format:

```json
[
  {"query": "c'est le troisième fix qu'on merge sur le flux de checkout ... et ça recasse ailleurs", "should_trigger": true},
  {"query": "Write a Quint spec for a simple two-phase commit protocol ...", "should_trigger": false}
]
```

Keep the set in the skill's `evals/` folder and commit it: it is the regression
test for every later change of the description. Keep raw run outputs out of the
commit unless the repo archives them.

## Staged protocol

Spend in steps; each step decides whether the next is worth it.

| Step | Calls | Purpose |
|---|---|---|
| 0. Measure one call per model | 1 per model | the real price of a call in this environment |
| 1. Pilot | 6 queries × 1 run | the harness works; pick 2 clear positives and the 4 hardest near misses |
| 2. Full pass | 20 × 1, sequential | a first complete picture |
| 3. Diagnose misses | 1 per miss | replay the miss and log the model's first action |
| 4. Repeat doubtful queries | 3 per doubtful query | noise or a real limit |
| 5. Other models | 20 × 1 per model | each model the skill is used from |
| 6. Optimize the description | many | only when steps 2 to 5 show a real gap |

A single run per query is an indication, not a rate. Say so when reporting.

Step 6 (`scripts/run_loop.py`) rewrites the description against a 60/40
train/test split. `skill-creator` recommends "pushy" descriptions to fight
undertriggering; for a skill meant to trigger rarely, read every proposed
description for loosened conditions before adopting it.

## Commands

Run from a throwaway directory holding an empty `.claude/`: the harness writes
its command files in the nearest ancestor that has one, which would otherwise be
the repository you work in.

```bash
D=~/.claude/plugins/marketplaces/anthropic-agent-skills/skills/skill-creator
SET=/abs/path/to/skill/evals/trigger-eval.json
T=$(mktemp -d) && mkdir "$T/.claude" && cd "$T"
PYTHONPATH=$D python3 $D/scripts/run_eval.py \
  --eval-set "$SET" --skill-path /abs/path/to/skill \
  --runs-per-query 1 --num-workers 1 --timeout 90 \
  --model claude-sonnet-5 --verbose \
  > result.json 2> stderr.log
grep -E 'Results|\[(PASS|FAIL)\]' stderr.log
cd / && rm -rf "$T"
```

Omit `--model` to use the configured default of `claude -p`.

Replay one query and print the model's first action (step 3):

```bash
T=$(mktemp -d) && mkdir -p "$T/.claude/commands" && cd "$T"
printf -- '---\ndescription: |\n  %s\n---\n\n# probe\n' "<description on one line>" \
  > .claude/commands/probe-skill.md
env -u CLAUDECODE claude -p "<query>" --output-format stream-json --verbose --max-turns 1 \
  | python3 -c '
import sys, json
for line in sys.stdin:
    e = json.loads(line) if line.strip().startswith("{") else {}
    if e.get("type") == "assistant":
        for c in e["message"]["content"]:
            if c["type"] == "tool_use":
                print("TOOL", c["name"], json.dumps(c["input"])[:120]); sys.exit()
            if c["type"] == "text" and c["text"].strip():
                print("TEXT", c["text"][:120])'
cd / && rm -rf "$T"
```

Measure the price of one call (step 0):

```bash
T=$(mktemp -d) && mkdir "$T/.claude" && cd "$T"
env -u CLAUDECODE claude -p "Reply: ok" --model claude-sonnet-5 --output-format json \
  | python3 -c 'import sys, json; d = json.load(sys.stdin); r = d[-1] if isinstance(d, list) else d; print(r["total_cost_usd"], r["usage"])'
cd / && rm -rf "$T"
```

## Pitfalls

- **`--num-workers` above 1 falsifies the result.** Every worker writes its own
  copy of the skill into the same `.claude/commands/`, so each `claude -p` sees
  several identical skills and may load another worker's copy, which counts as a
  miss. Measured with 3 workers: 3 of 8 positives triggered; sequentially, 8 of 8.
  Negatives are unreliable too: a false trigger on a neighbour's copy counts as a
  pass. The default is 10 workers. Always pass `--num-workers 1`.
- **A timeout counts as a miss.** The default is 30 s; a cold `claude -p` with
  many plugins can take longer. Use `--timeout 90`. Not measured how often 30 s
  is exceeded.
- **First action only.** A model that runs `Bash` or `Glob` before loading the
  skill scores a miss. When a miss surprises you, replay it (step 3) before
  editing the description.
- **Global config leaks in.** `claude -p` loads your user settings, plugins,
  skills and MCP servers. That is realistic, since the skill competes with the
  others you have installed, but it makes results machine-specific: record the
  machine and the date.
- **One run is noise-prone.** In the example below, the same query missed once and
  triggered on replay.

## Cost

Measured on 2026-09-26, one call with an empty `.claude/`, about 26,000 context
tokens (about 14,700 written to cache and 11,400 read), 4 output tokens:

| Model | One call |
|---|---|
| `claude-opus-5-5` | 0.120 USD |
| `claude-sonnet-5` | 0.062 USD |

A trigger-eval call stops at the first tool call, so its cost is almost all
context read: estimate a pass as calls × the measured price, and re-measure when
the installed plugins change.

## Worked example: state-audit

A skill meant to trigger rarely: only when a symptom crosses several
components, after two symptom fixes on one flow, or on an explicit audit
request. Set: `state-audit/evals/trigger-eval.json` in this repository.

| Step | Calls | Result |
|---|---|---|
| Pilot, 3 workers | 6 | 5/6: one positive missed, then triggered on replay |
| Full pass, 3 workers | 20 | 15/20: 5 positives missed |
| Replay of the 5 misses | 5 | all 5 triggered: the harness, not the description |
| Full pass, sequential, Opus 5.5 | 20 | 20/20 |
| Full pass, sequential, Sonnet 5 | 20 | 19/20: one positive missed, the least explicit query |

About 8 USD in all, of which about 2.8 USD went to the invalid parallel pass.
Negatives were 12/12 on both valid passes, including the vocabulary trap (a
Quint spec for a new protocol).
