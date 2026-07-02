---
type: journal
updated: YYYY-MM-DD
---

# Journal

Dated activity log, newest at the bottom (append-only). This file holds the
recent window only. Once it grows past the seal threshold, `archive-journal.sh`
moves the oldest whole months into `log/YYYY-MM.md` and lists them under Archives.

Entry heading format: `## YYYY-MM-DD short title` — the date drives archiving, so
keep it as the first thing on the heading line.

## YYYY-MM-DD first entry

Replace this with your first real entry. One block per event; keep it short and
link to a `raw/` capture or a `pages/` synthesis instead of pasting long content.

## Archives

<!-- Managed by archive-journal.sh. Sealed months are listed here, newest first. -->
