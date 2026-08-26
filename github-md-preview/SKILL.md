---
name: github-md-preview
description: >
  USER-INVOKED ONLY. Use only when the user explicitly invokes $github-md-preview,
  /github-md-preview, or explicitly names this skill. Never invoke implicitly.
  Preview a local markdown file with real GitHub rendering (official markdown
  API plus GitHub CSS) served on localhost, without committing or pushing, to
  check mermaid diagrams, tables, or alerts before pushing. Requires network
  access to api.github.com and the gh CLI.
disable-model-invocation: true
---

# GitHub Markdown Preview

Render a local markdown file exactly as github.com would, on localhost. The
`gh-markdown-preview` gh extension sends the file to GitHub's official
markdown API and wraps the result in CSS extracted from github.com. Live
reload included. Nothing is committed, pushed, or leaves the machine except
the file content sent to the rendering API.

## One-Time Setup

Check, then install if missing:

```bash
gh extension list | grep markdown-preview || gh extension install yusukebe/gh-markdown-preview
```

Only `gh` is required. No token needed; the markdown API is public but rate
limited.

## Serve the Preview

1. Start the server in the background so it survives across turns:

    ```bash
    gh markdown-preview README.md --disable-auto-open
    ```

2. Read the server output for the URL. Default: `http://localhost:3333/`. A
   taken port increments automatically.
3. Give the URL to the user. `--disable-auto-open` stops the tool from
   grabbing the browser; drop the flag to auto-open instead.

The server watches the file and live-reloads the page on every save. Leave it
running while the user reviews. Stop it when they are done: kill the
background task or Ctrl-C.

## Options

| Flag                          | Effect                     |
| ----------------------------- | -------------------------- |
| `--dark-mode` / `--light-mode` | Force a color scheme       |
| `-p <port>`                   | Fixed port (default 3333)  |
| `--disable-reload`            | No live reload             |

One served file per invocation. Pipe stdin for one-off snippets:

```bash
echo "# Hello" | gh markdown-preview
```

## What This Validates

GitHub renders GFM: tables, task lists, alerts (`> [!NOTE]`), emoji
shortcodes, and ` ```mermaid ` fences. If a mermaid diagram shows an error
block in the preview, the fence syntax is wrong; fix the markdown source and
the page reloads. A clean preview means a clean render after push.

## Limits and Alternatives

- Needs network: rendering happens through `api.github.com`.
- Unauthenticated rate limits apply; for heavy use, Grip
  ([joeyespo/grip](https://github.com/joeyespo/grip)) offers the same
  API-based approach with credential support.
- Fully offline alternatives: [ghore](https://github.com/p-balu/ghore) (CLI,
  mermaid support) and [GHMD](https://github.com/Ubpa/ghmd) (VS Code
  extension).
