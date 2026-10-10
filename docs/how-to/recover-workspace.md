# Recover an existing workspace

[Documentation](../README.md) / How-to

Stop mutating Spacer commands and back up the entire `.spacer` directory before
manual repairs. Keep project directories and Git worktrees in place.

## Legacy configuration

Unversioned `config.json` with valid absolute paths loads without changing the
file. Its next successful mutation writes schema version 1 atomically, preserving
registrations, tracking entries, custom paths, and valid saved context. Unknown
schemas or fields are rejected rather than discarded.

Relative legacy paths are ambiguous because old commands interpreted them against
the invocation directory. Verify each directory, then replace the stored path
with its absolute location in your backed-up configuration. Do not guess a base.
Resolve duplicate registrations that point at the same canonical project directory.

## Former Git backend

The presence of `.spacer/state.json` blocks loading, even if `config.json` also
exists. Back up both files. Inspect the saved context and use `git worktree list`
in each repository to inventory actual checkouts. Reconcile intended registrations
and active context in `config.json`; preserve any metadata-only tracking entries.
Archive `state.json` outside `.spacer` only after reconciliation. Spacer does not
silently choose between the two sources or move/delete their project files.

## Failed mutation

A conflict means another writer changed the registry. Reopen before retrying.
Directory or Git creation can have succeeded before saving failed; inspect the
reported path first. Do not delete a repository to retry registration. An error
reporting a failed directory sync means replacement may already be visible:
reopen and inspect before retrying. Temporary files left by interruption are not
read as configuration; never remove `registry.lock` while a process is running.

Invalid relationships or names require repair of the identified record. Stale
saved selections clear in memory and persist on the next successful mutation.

Git worktree start requires an existing commit and a new branch. If Git reports
a partial failure, inspect `git worktree list` and the named branch before
retrying; Spacer does not reset branches or prune worktrees automatically. Finish
refuses modified, untracked, ignored-file-containing, locked, or primary checkouts.
A clean worktree can be closed with unmerged commits because its branch remains.
Avoid editing a checkout concurrently with finishing it.
