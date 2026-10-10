# Configuration reference

[Documentation](../README.md) / Reference

## Root and context

| Variable | Behavior |
| --- | --- |
| `SPACER_ROOT` | Workspace root when set; otherwise the current directory. Use an absolute path. |
| `SPACER_SPACE` | Nonempty value overrides the saved active space. |
| `SPACER_PROJECT` | Nonempty value overrides the saved active project where a command accepts that default. |
| `SHELL` | Selects shell integration output when the shell argument is omitted. |

Explicit command arguments take precedence over environment and saved defaults.
No workspace ancestor search or project inference from the current checkout is
performed. Setting an active project does not change the active space. Saved
context is shared by all sessions using that configuration. Space/project names
in environment overrides are returned as-is; operations may fail when resolving
an unknown name.

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
name, project, and space. Active projects store only a name. There is no schema
version or migration mechanism.

Missing configuration loads as empty state. Mutation followed by save creates
metadata as needed, even without an explicit init. Invalid JSON is an error.
`workspace init` loads configuration first, then checks for `.spacer` existence.
Malformed data is reported without being overwritten.

## Persistence limits

Writes replace the file directly, without atomic replacement or locking. Avoid
concurrent mutating commands and keep a backup before significant edits.
Deletion does not remove child records or clear context. Switching spaces does
not reset the saved project. There is no command to clear saved context.

Custom paths can point outside the root. Names and paths are not comprehensively
validated, and printed navigation paths are not checked for existence. These are
current constraints, not guarantees of safe arbitrary filesystem operations.

The Git adapter performs Git operations only; it does not own registration or
active context. The former Git backend's `.spacer/state.json` is not read or
migrated. See [architecture](../explanation/architecture.md) and the
[Rust API migration reference](rust-api.md).
