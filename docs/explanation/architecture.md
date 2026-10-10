# Workspaces and adapters

[Documentation](../README.md) / Explanation

Spacer's organizing idea is a hierarchy: a workspace contains spaces, spaces
group projects, and projects have changes. This lets a developer use meaningful
names rather than remember every checkout's filesystem path.

The intended Git workflow associates a project with a repository and a change
with a worktree. The current application implements the organizational records;
its Git operations exist separately in the library. A change listed by the CLI
is therefore not evidence that a corresponding checkout exists.

## Code boundaries

The Rust workspace has two packages. The `spacer` binary contains CLI parsing,
command handlers, row mapping, and terminal rendering. `spacer-core` defines
spaces, projects, changes, a workspace facade, and store traits.

`Workspace<B>` delegates operations to a backend implementing space, project,
and change stores plus persistence. The adapters currently differ:

| Adapter | State and side effects |
| --- | --- |
| Config | JSON tracking plus directory creation; deletion forgets records. Used by CLI/TUI. |
| Git | Discovers directories/repositories/worktrees; creates repositories and worktrees; persists active context separately. Project deletion recursively removes files. |
| Memory | In-memory state for tests, with no filesystem writes or environment overrides. |

These adapters are not yet interchangeable in their lifecycle semantics. In
particular, connecting the Git adapter changes the meaning of deletion and
finish. Library behavior must not be inferred from the CLI's metadata-only
commands. The TUI currently consumes a cloned configuration snapshot rather than
an abstract workspace view.

## What belongs to Git

Commits, branches, merges, remote synchronization, and actual linked checkouts
are Git concepts. Spacer's current tracking layer does not verify or control
them. Users can initialize a repository or create worktrees with Git independently,
then record the relevant project directory and change names in Spacer.

This distinction explains why `change finish` currently removes a record while
leaving branch and checkout cleanup to the user. The
[CLI reference](../reference/cli.md) describes application behavior;
[configuration](../reference/configuration.md) describes where that state lives.
