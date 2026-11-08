# Proving a change

Each stage catches what the previous one cannot see. Run the stages that fit the lever.

1. **Local tests.** When the change adds decision logic (a skip, a mode switch), put it in pure functions and test them. Break the key condition on purpose: a mutant that passes the tests means the tests prove nothing, so add the case that turns it red.
2. **Live read-only trial.** For decision logic that reads an API, run it against the real API with a real token, in a temporary worktree of the target branch, and read what it decides. Run it again with a broken token: the fallback must take the full path.
3. **Static checks.** The repo's hooks, its typecheck and lint, a workflow linter on every changed workflow and action.
4. **Run on the branch.** Trigger the changed workflows on the branch before opening a pull request, to measure without merging. Know which runs write the cache under the repository's policy and the provider's scoping.
5. **Pull request.** Review the diff first. The body carries a before and after table with each number's conditions, the decisions, and what is left open.
6. **Wait on CI.** Watch with the provider CLI's watch command or poll the run API at a fixed interval, in the background. Stop on every terminal state (success, failure, cancelled, closed without merge) and on a timeout, since a job queued for a runner that never comes reaches no terminal state; then follow the runs the merge triggers.
7. **After the merge.** Read the first run that the merge triggers: the cache published, the reuse count, the skip line naming the run it trusted. Put the numbers in the final before and after table.
8. **Report.** Per lever: before and after, in wall time and in billed minutes, each number with its conditions. Totals per change (all jobs one merge triggers) are estimates when built from single runs, and say so.
