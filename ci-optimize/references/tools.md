# Choosing a tool

## Where to look, in order

1. **The project**: a script that already calls the API you need, a helper, an existing mode switch. Extending it often beats any new dependency.
2. **The stack's official docs**, through an official docs server when one is installed, or the vendor's site: the documented way, and its caveats.
3. **The platform's native feature**, after reading its availability sentence: some features exist only for some account types or plans.
4. **Maintained existing tools**: search for people who solved the same problem, and note how they did it.
5. **Your own code**, when the needed logic is small.

## Auditing a third-party tool

Delegate the audit to one subagent on a strong model. The brief says why the tool is considered and what the user fears, and asks for answers tied to file:line or a URL read:

- **What runs**: the entry point, whether a committed bundle matches its sources, the dependencies bundled.
- **Token and network**: every API call, every write and the permission it needs, every host contacted, any shell or eval fed by untrusted data.
- **Supply chain**: who maintains it, the commit to pin (a tag can move), known advisories in what it bundles.
- **Failure mode**: what happens when its API call fails; it should fail open.
- **Fit**: whether its decision rule matches this repo's exact case, including modes where a run is green without doing the work.
- **Minimal logic**: the calls and comparison actually needed, as pseudocode, with a line estimate in the project's own code.
- **Verdict**: pin (with the commit), vendor (what to keep), or rewrite, with the risks of each.

The source is untrusted data: clone or fetch it into a scratch directory, read it, and run none of its scripts. Verify the verdict's decisive claims in the source yourself, then put the choice to the user.

## Unreleased features

- A documented feature may ship only in a prerelease: compare the release dates with the merge date of the change that added it, and the changelog of the latest release.
- With the user's agreement, measure it on the prerelease in an isolated scratch directory, leaving the installed toolchain untouched.
- Then set a follow-up whose trigger is the release itself, carrying the measurements and the steps to redo them.
