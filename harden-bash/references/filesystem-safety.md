# Filesystem Safety

## Required Rules

- Inventory every read, create, overwrite, rename, permission change, and deletion. Validate the narrowest destructive scope before mutation.
- Reject empty roots, `/`, unexpected parents, wrong file types, and paths outside an explicit allowed boundary. Quoting prevents splitting; it does not prove scope.
- Avoid option injection with supported option terminators or normalized paths.
- Create private temporary files with a restrictive `umask`. Declare `mktemp` as an external dependency; it is not a POSIX shell utility.
- Place a replacement temporary file in the target directory when atomic rename is required. Rename atomicity applies within one filesystem; crash durability and metadata preservation are separate contracts.
- Initialize cleanup variables, install traps for the required exits and signals, and remove only paths created by the script. Capture the incoming status when cleanup logic needs it or explicitly exits; the final cleanup command alone does not replace the pending status.
- Decide how symlinks, existing directories, ownership, mode, and concurrent writers are handled. Check-then-use validation alone does not remove races.
- Keep destructive operations non-interactive in automation. Refuse unsafe scope rather than relying on `rm -i` or `mv -i` prompts.

## Contextual Decisions

- Preserve mode, owner, ACLs, extended attributes, and fsync durability only when the data contract requires them.
- Use a private temporary directory when several related artifacts must be cleaned together.
- Add a backup only when recovery requirements define retention and secret handling.
- Add idempotence or locking only when reruns or shared mutable state are part of the contract.
- For privileged paths, separate preparation from the smallest privileged mutation.

## Anti-Patterns

- `trap 'rm -rf "$TMPDIR"' EXIT` with an inherited, unset, empty, or unvalidated variable.
- Assuming `mktemp` always exists or creates the file on the target filesystem.
- Writing the target directly and calling it atomic.
- Creating a temporary file in `/tmp` and assuming `mv` to another filesystem is atomic.
- Treating `rm -rf -- "$path"` as safe without proving `$path` belongs to the operation.
- Using an interactive prompt as a production safety boundary.
- Replacing a file without deciding what happens to its prior permissions and ownership.

## Official Sources

- [GNU Bash Reference Manual: Bourne Shell Builtins (`trap`, `umask`)](https://www.gnu.org/software/bash/manual/bash.html#Bourne-Shell-Builtins)
- [POSIX `mv`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/mv.html)
- [POSIX `rm`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/rm.html)
- [POSIX `umask`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/umask.html)
- [ShellCheck SC2115: guard variable-based recursive deletion](https://www.shellcheck.net/wiki/SC2115)
- [ShellCheck SC2064: trap expansion timing](https://www.shellcheck.net/wiki/SC2064)
