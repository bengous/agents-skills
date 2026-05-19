# Official Docs And External References

Use official Claude Code docs for live runtime contracts. This skill provides
workflow and examples, not a frozen copy of the product documentation.

## Claude Code

| Need | Source |
|---|---|
| Hook events, handlers, input/output, exits | `https://code.claude.com/docs/en/hooks` |
| Guided setup and common use cases | `https://code.claude.com/docs/en/hooks-guide` |
| Settings files and config precedence | `https://code.claude.com/docs/en/settings` |
| Full docs map for agents | `https://code.claude.com/docs/llms.txt` |
| Claude Code changelog | `https://code.claude.com/docs/en/changelog` |

Check the hooks reference before relying on event lists, handler fields,
timeouts, decision JSON, matcher behavior, or newly added hook events.

## Shape Inspiration

GBrain is not an upstream for this skill, but its agent-facing docs are a useful
structure reference:

- Repository: `https://github.com/garrytan/gbrain`
- Agent entrypoint: `https://raw.githubusercontent.com/garrytan/gbrain/master/AGENTS.md`
- Docs map: `https://raw.githubusercontent.com/garrytan/gbrain/master/llms.txt`
- Skill resolver: `https://raw.githubusercontent.com/garrytan/gbrain/master/skills/RESOLVER.md`

Borrow the pattern of concise routing and owned scaffolded skills. Do not borrow
GBrain's product domain, metrics, or memory architecture for Claude Code hooks.

## Refresh Routine

For current docs, prefer direct fetches:

```bash
curl -fsSL https://code.claude.com/docs/llms.txt
curl -fsSL https://code.claude.com/docs/en/hooks
curl -fsSL https://code.claude.com/docs/en/settings
```

Then update only the small references that changed. Do not vendor large scraped
HTML into this skill.
