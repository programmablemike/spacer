# CLI reference

[Documentation](../README.md) / Reference

Syntax: `spacer [COMMAND]`. Square brackets indicate optional arguments;
uppercase words are placeholders. Use a subcommand's `--help` for built-in help,
and `spacer --version` for the version. With no command, Spacer opens the TUI.

Workspace commands share the application service and versioned JSON registry.
Repository creation and worktree commands invoke Git. Explicit legacy tracking
operations use metadata only.

## Workspace and spaces

| Command | Effect |
| --- | --- |
| `workspace init` | Initialize the explicit root or current directory. Preserve valid existing state; refuse malformed state. |
| `space create NAME [--path PATH]` | Create/register a directory; default `<root>/<name>`. |
| `space list` | List spaces and mark the effective active space. |
| `space use NAME` | Save the active space; clear the saved project if space changes. |
| `space current` | Print the active space, or report none selected. |
| `space go [NAME]` | Print the existing named/active space path; navigate with shell integration. |
| `space unregister NAME` | Remove a registration with no projects; preserve its directory and clear its selection. |
| `space delete NAME` | Deprecated alias for unregister. |

## Projects

| Command | Effect |
| --- | --- |
| `project create NAME [--space SPACE] [--path PATH]` | Initialize Git in a new/empty directory and register it; default `<space-path>/<name>`. No initial commit. |
| `project register NAME --path PATH [--space SPACE]` | Register an existing primary non-bare Git repository without changing its files. |
| `project list [--space SPACE]` | Filter by explicit/active space; list all if neither exists. |
| `project use NAME [--space SPACE]` | Save the active project and space together. |
| `project current` | Print the active project, or report none selected. |
| `project go [NAME] [--space SPACE]` | Print the existing named/active project path; navigate with shell integration. |
| `project unregister NAME [--space SPACE]` | Remove registration and affected selection; preserve repositories, branches, and worktrees. Refuse while legacy tracking entries remain. |
| `project delete NAME [--space SPACE]` | Deprecated alias for unregister. |

There is no recursive project/space deletion command. Selecting defaults uses
`space use` and `project use`; session overrides never change saved selections.
See [configuration](configuration.md).

## Changes

| Command | Effect |
| --- | --- |
| `change start NAME --project PROJECT [--space SPACE]` | Create a new branch and linked worktree from the project's HEAD. Require an initial commit. |
| `change list [--project PROJECT] [--space SPACE]` | Query actual linked worktrees, including external paths and detached checkouts. Omitting project lists across projects in the explicit/active space, or all spaces if none selected. |
| `change finish NAME --project PROJECT [--space SPACE]` | Remove a clean linked worktree; retain its branch and commits. |
| `change list --tracked [--project PROJECT] [--space SPACE]` | List legacy metadata tracking entries without querying Git. |
| `change finish NAME --tracked --project PROJECT [--space SPACE]` | Remove only a legacy tracking entry. |

Start refuses existing branches or target directories. New checkouts live under
`<root>/.spacer/worktrees/<space>/<project>/<branch>`; branch names can contain
slashes. Existing Git checkout paths remain authoritative.

Finish refuses modified, untracked, ignored-file-containing, locked, primary,
and detached checkouts. It has no force option. Git branches survive even when
commits have not been merged. Do not edit a checkout concurrently with finishing
it. See [recovery](../how-to/recover-workspace.md) after any partial failure.

## Paths and navigation

| Command | Effect |
| --- | --- |
| `path space [NAME]` | Print the existing named/active space path. |
| `path project [NAME] [--space SPACE]` | Print the existing named/active project path. |
| `path change NAME [--project PROJECT] [--space SPACE]` | Print the existing Git checkout path for the named branch. |
| `go SPACE[/PROJECT[/CHANGE]]` | Resolve a fully named target; navigate with shell integration. The change can contain slashes. |

Path output contains only the path, without a trailing newline. Resolution errors
return nonzero with no path on stdout. Detached worktrees require Git directly.

## Shells and completions

`spacer shell [--shell PATH]` launches a managed interactive shell on macOS/Linux.
It requires an initialized workspace and terminal, exports an absolute root, and
refuses nesting. Bash/zsh receive navigation, a prompt indicator, and completions.
POSIX sh receives environment variables only. Windows has no managed shell.

`spacer shell integration [SHELL]` emits a navigation wrapper for bash/zsh/sh,
without opening a workspace. The omitted shell comes from the basename of
`SHELL`, falling back to sh. Unsupported shells fail. The wrapper intercepts
`go`, `space go`, and `project go`; failure preserves the current directory.

`spacer completions SHELL` emits bash/zsh/fish completion scripts without opening
a workspace. See [shell setup](../how-to/shell-navigation.md) for startup details
and Windows path navigation.

## Terminal browser

| Key | Action |
| --- | --- |
| `Tab` | Cycle spaces/projects/changes tabs |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `q` / `Ctrl-C` | Exit |

The TUI loads the registry and registered repositories' Git worktrees before
entering raw mode. It displays detached checkout paths, empty states, and legacy
tracking separately. Read errors abort startup. It restores terminal modes on
exit and panic. It is a read-only snapshot: restart to refresh. It cannot mutate
records or change the parent shell's directory.
