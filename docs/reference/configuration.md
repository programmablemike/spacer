# Configuration reference

[Documentation](../README.md) / Reference

## Root and context

| Variable | Behavior |
| --- | --- |
| `SPACER_ROOT` | Workspace root when set; otherwise the current directory. Use an absolute path. |
| `SPACER_SPACE` | Nonempty value overrides the saved active space. |
| `SPACER_PROJECT` | Nonempty value overrides the saved active project where a command accepts that default. |
| `SHELL` | Selects shell integration output when the shell argument is omitted. |

Explicit command arguments take precedence over valid environment and saved defaults.
Session overrides are validated when opening the workspace, even for commands
with explicit flags; unknown overrides fail with instructions to correct or
unset them. No workspace ancestor search or checkout inference is performed.

The saved `active_space` and `active_project` form a pair. Selecting a project
sets both fields; changing the saved space clears its project selection. A space
override cannot reinterpret a saved project in another space, even if both spaces
have a project with the same name. A project override is bound to the resolved
space when the workspace opens. Session overrides never overwrite saved defaults.
`project go --space OTHER` requires an explicit project name when OTHER is not
the selected space. Saved defaults remain shared by all sessions.

`change start` and `change finish` require `--project`; `change list` without it
lists across projects. Relative root/custom paths are not normalized to absolute
paths, so their interpretation can change with the invocation directory.

## Configuration file

The CLI and TUI use `<root>/.spacer/config.json`. Example with an absolute root:

```json
{
  "spaces": [{"name": "personal", "path": "/home/user/code/personal"}],
  "projects": [{"name": "website", "space": "personal", "path": "/home/user/code/personal/website"}],
  "changes": [{"name": "feature-x", "project": "website", "space": "personal"}],
  "active_space": "personal",
  "active_project": "website"
}
```

The three arrays are required on deserialization. Active selections are optional
and omitted when unset. Projects are identified by name and space; changes by
name, project, and space. The two active fields jointly identify the selected project. Schema version 1 is written on successful mutation. Unversioned configurations
are read as legacy version 0; future versions and unknown top-level fields are
rejected without rewriting them.

Missing configuration loads as empty state. Mutation followed by save creates
metadata as needed, even without an explicit init. Invalid JSON is an error.
`workspace init` loads configuration first, then checks for `config.json` existence.
Malformed data is reported without being overwritten.

## Invariants and recovery

Space/project names must be nonempty, non-hidden portable directory components.
Separators, control characters, Windows reserved characters/device names, and
trailing dots/spaces are rejected on every platform. Internal spaces are allowed.
Change names allow slash-separated branches such as `feat/login`, but reject
traversal, empty components, Git-special syntax, and nonportable path components.
These checks apply before directory creation or Git effects and when loading
existing registry records.

Spaces with registered projects cannot be removed. Projects with tracked changes
cannot be removed until those tracking entries are finished. Removing a selected
record clears its saved selection; files and Git worktrees are preserved.

On load, stale active selections clear in memory without writing the file; the
next successful mutation saves the corrected defaults. Duplicate records,
invalid names, and orphaned projects/changes fail before any mutation. Back up
`config.json`, then repair the identified record or restore its missing parent.
Spacer does not discard orphaned records automatically.

## Persistence limits

Writes use a flushed temporary file and atomic replacement under an OS file
lock. A byte-for-byte comparison with the loaded configuration rejects stale
writes: reopen and retry after a conflict. The lock is released on process exit;
do not delete `registry.lock` while Spacer is running. Interrupted temporary
files are ignored. Back up configuration before manual edits.
There is no separate command to clear saved context.

Custom paths can point outside the root. Path normalization, symlink containment, and canonical repository uniqueness
are not enforced, and printed navigation paths are not checked for existence. These are
current constraints, not guarantees of safe arbitrary filesystem operations.

The Git adapter performs Git operations only; it does not own registration or
active context. The former Git backend's `.spacer/state.json` is not read or
migrated. See [architecture](../explanation/architecture.md) and the
[Rust API migration reference](rust-api.md).
