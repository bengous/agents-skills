# looper

Co-design bounded Codex loops. The skill interrogates, challenges, and
proposes; you decide; nothing is emitted before your explicit ok.

## Invoke

```text
$looper design a loop for fixing review comments until CI passes
$looper make a stacked PR loop for this migration
$looper review this loop for runaway risk
$looper what could I loop in this repo?
```

Outputs after agreement: a raw `/goal` payload (long ones via `goalify`), a
runner spec for heavy loops (ralf-loop shape), or a durable
`.agents/loops/<slug>.md` artifact.

See `SKILL.md` for the full contract.
