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
| `workspace init` | Write empty configuration at the root if `.spacer` does not exist. If it exists, report already initialized after loading configuration; malformed configuration is an error. |

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
| `project create NAME [--space SPACE] [--path PATH]` | Record a project and create its directory; default path is `<space-path>/<name>`. Does not initialize Git. |
| `project list [--space SPACE]` | List projects filtered by explicit/active space; list all if neither exists. |
| `project use NAME [--space SPACE]` | Save the active project and its space together. |
| `project current` | Print active project or a message that none is selected. |
| `project go [NAME] [--space SPACE]` | Print the named/active project's path in the resolved space. |
| `project delete NAME [--space SPACE]` | Remove a project without tracked changes, preserving files and clearing its selection. |

## Changes

| Command | Effect |
| --- | --- |
| `change start NAME --project PROJECT [--space SPACE]` | Add a tracking entry; does not create a branch or worktree. |
| `change list [--project PROJECT] [--space SPACE]` | List changes filtered by provided project and explicit/active space. Omitting project does not apply the active project. |
| `change finish NAME --project PROJECT [--space SPACE]` | Remove the tracking entry; preserve Git branches, worktrees, and files. |

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
