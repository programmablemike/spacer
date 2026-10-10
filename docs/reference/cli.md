# CLI reference

[Documentation](../README.md) / Reference

Syntax: `spacer [COMMAND]`. Square brackets below indicate optional arguments;
uppercase words are placeholders. `spacer --help` and each subcommand's `--help`
provide built-in help. `spacer --version` prints the package version.

With no command, Spacer opens the read-only TUI. All commands below use the
shared workspace service with JSON persistence. Tracking commands do not invoke Git.

## Workspace

| Command | Effect |
| --- | --- |
| `workspace init` | Write empty configuration at the root if `config.json` does not exist. If it exists, report already initialized after loading configuration; malformed configuration is an error. |

## Spaces

| Command | Effect |
| --- | --- |
| `space create NAME [--path PATH]` | Record a space and create its directory if needed; default path is `<root>/<name>`. |
| `space list` | List spaces and mark the active name. |
| `space use NAME` | Save the active space; clear the saved project when the space changes. |
| `space current` | Print active space or a message that none is selected. |
| `space go [NAME]` | Print named/active space path. |
| `space delete NAME` | Remove an empty space record, preserving files and clearing its selection. |

## Projects

| Command | Effect |
| --- | --- |
| `project create NAME [--space SPACE] [--path PATH]` | Initialize Git in a new/empty directory and register it; default path is `<space-path>/<name>`. No initial commit. |
| `project list [--space SPACE]` | List projects filtered by explicit/active space; list all if neither exists. |
| `project use NAME [--space SPACE]` | Save the active project and its space together. |
| `project current` | Print active project or a message that none is selected. |
| `project go [NAME] [--space SPACE]` | Print the named/active project's path in the resolved space. |
| `project delete NAME [--space SPACE]` | Remove a project without tracked changes, preserving files and clearing its selection. |

## Changes

| Command | Effect |
| --- | --- |
| `change start NAME --project PROJECT [--space SPACE]` | Create a new branch and linked worktree from HEAD. |
| `change list [--project PROJECT] [--space SPACE]` | List changes filtered by provided project and explicit/active space. Omitting project does not apply the active project. |
| `change finish NAME --project PROJECT [--space SPACE]` | Remove a clean linked worktree; retain its branch and commits. |

Deletion refuses parents with tracked children. Finish tracked changes before
removing their project, and remove projects before removing their space.
See [configuration](configuration.md) for context resolution.

## Shell integration

`spacer shell [SHELL]` emits a wrapper for `zsh`, `bash`, or `sh`. If omitted,
SHELL is inferred from the basename of the `SHELL` environment variable, falling
back to `sh`. Unsupported values fail. The wrapper turns `space go` and
`project go` into directory changes. See [setup](../how-to/shell-navigation.md).

## Terminal browser

| Key | Action |
| --- | --- |
| `Tab` | Cycle spaces/projects/changes tabs |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `q` / `Ctrl-C` | Exit |

The TUI reads a snapshot on startup. It cannot edit records, refresh live state,
or navigate the parent shell. Restart it to reload the configuration.

## Git lifecycle and compatibility

`project register NAME --path PATH [--space SPACE]` registers an existing primary
non-bare repository. `project unregister` and `space unregister` preserve files;
legacy `delete` commands are aliases. Spaces must have no registered projects.

`project create` initializes Git in a new/empty directory. Make the first commit
with Git before `change start`. Change listing reads actual linked worktrees,
including external and detached checkouts. Finish refuses primary, dirty,
ignored-file-containing, locked, and detached worktrees, with no force option.

`change list --tracked` and `change finish NAME --project PROJECT --tracked` are
explicit metadata-only operations for legacy records. They do not touch Git.
