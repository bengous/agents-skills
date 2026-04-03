# Authoring — Write and Modify Hooks

Create new hooks or modify existing ones.

## Choose your path

| Goal | Reference |
|------|-----------|
| Write a standalone hook (bash, python, or bun) | [standalone-patterns.md](standalone-patterns.md) |
| Write a hook using the typed runtime with `defineHook` | [typed-runtime-patterns.md](typed-runtime-patterns.md) |
| Safely modify an existing hook | [modifying-existing-hooks.md](modifying-existing-hooks.md) |

## Hook authoring fundamentals

Every hook, regardless of implementation strategy, follows the same contract:

1. **Receives** event JSON on **stdin**
2. **Decides** by setting the **exit code**: 0 (allow), 1 (error), 2 (block)
3. **Communicates** via **stderr** (blocking reasons, warnings) and **stdout** (structured JSON responses)

The event JSON always includes common fields (`session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`) plus event-specific fields documented in [events](../events/index.md).

## After writing a hook

1. Register it in settings — see [config](../config/index.md)
2. Test it — see [testing](../testing/index.md)
