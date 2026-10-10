# Rust API and migration reference

[Documentation](../README.md) / Reference

## Packages and dependencies

| Package | Public surface | Dependencies |
| --- | --- | --- |
| `spacer-core` | Models, `Workspace`, outbound ports, `Error`/`Result` | Standard library only |
| `spacer-data` | `JsonRegistry`, `LocalFilesystem`, `GitCli` | `spacer-core`, Serde, serde_json |
| `spacer` | CLI/TUI executable and composition root | Both libraries plus presentation dependencies |

Presentation modules depend on the core API. Only the executable's composition
root imports concrete data adapters. There is no Tauri dependency.

## Constructing a workspace

```rust
use spacer_core::{ContextOverrides, Workspace};
use spacer_data::{GitCli, JsonRegistry, LocalFilesystem};
use std::path::PathBuf;

fn open(root: PathBuf) -> spacer_core::Result<Workspace> {
    let registry = JsonRegistry::new(&root);
    Workspace::open(
        root,
        registry,
        LocalFilesystem,
        GitCli,
        ContextOverrides::default(),
    )
}
```

`Workspace::open` loads through the registry. It receives dependencies implementing
`Registry`, `Filesystem`, and `Git`, owns them for its lifetime, and never reads
environment variables. Pass `ContextOverrides` explicitly to apply session
space/project selections. Nonempty overrides take precedence over saved context;
metadata writes do not persist overrides. Unknown session overrides fail during
open. Space and project selections are resolved as a pair; a project override
is bound to its resolved space. See [configuration](configuration.md) for name
rules, registry validation, and recovery behavior.

`initialize()` returns `true` when it writes the initial configuration and `false`
when metadata already exists. `root()` returns the configured path. `snapshot()`
returns an owned copy with effective context for presentation use. Mutating that
copy does not change the workspace.

## Use cases

| Methods | Effects |
| --- | --- |
| `spaces`, `projects`, `changes` | Query loaded tracking state, with optional filters. |
| `create_space`, `create_project` | Create directories and persist records. |
| `delete_space`, `delete_project` | Reject tracked children, remove the record, and clear affected saved context; never delete files. |
| `set_active_space`, `set_active_project` | Validate the target and save a coherent space/project pair; changing space clears the project. |
| `active_space`, `active_project`, `active_project_in` | Read effective context; `active_project_in(space)` returns a default only for the selected space. |
| `start_change`, `finish_change` | Persist addition/removal of metadata tracking entries. |
| `create_git_project` | Initialize a new/empty directory as Git and register it; create no commit. |
| `register_project` | Validate and register an existing primary non-bare repository without modifying its files. |
| `worktrees` | Query actual Git worktrees for a registered project. |
| `start_worktree` | Create a new branch/worktree from an explicit base such as `HEAD`; no tracking entry is added. |
| `close_worktree` | Find a branch-backed checkout by its observed Git path and remove it without force; refuse the primary checkout. Retain its branch. |

Mutating tracking use cases save before returning success; there is no public
`save()` step. Query results describe the loaded registry snapshot and do not
refresh it automatically. Reopen to see external registry changes. Git worktree
queries run against Git on each call.

New worktrees use `<root>/.spacer/worktrees/<space>/<project>/<branch>`, outside
the source tree. Existing/custom Git worktree paths remain authoritative.
The application service rejects empty and traversing worktree paths; the Git
adapter checks branch syntax. This is not a complete alpha path-validation policy.
Externally created worktrees retain their actual locations. Detached worktrees
are observable but cannot be closed by the branch-name use case.

## Outbound ports

- `Registry`: `load`, `save`, and `is_initialized`; load/save domain snapshots.
- `Filesystem`: `normalize`, `is_dir`, `is_empty_or_missing`, `create_dir_all`; no project-removal policy or recursive deletion.
- `Git`: `validate_repository`, `is_clean`, `initialize`, `add_worktree`, `worktrees`, and `remove_worktree`; no
  registration or active-context policy.

Implementations must be `Send + Sync` and return `spacer_core::Result`.
`Error` categories are `NotFound`, `AlreadyExists`, `InvalidOperation`,
`Persistence`, `Filesystem`, and `Git`. Each carries diagnostic text; callers
should match categories rather than parse that text.

## Migrating from the previous library API

This is a breaking library refactor. Package versions are unchanged pending the
next release decision.

| Previous API | Replacement |
| --- | --- |
| `Workspace<Backend>` and implicit `Workspace::open(&root)` | `Workspace::open(root, registry, filesystem, git, context)` |
| `Backend`, `SpaceStore`, `ProjectStore`, `ChangeStore` | Separate outbound ports and the `Workspace` use-case API |
| `ConfigAdapter` / exported `Config` | `spacer_data::JsonRegistry` / `WorkspaceSnapshot` |
| `GitAdapter` as a substitute backend | `spacer_data::GitCli` injected alongside registry/filesystem adapters |
| `MemoryAdapter` | Implement small port fakes in tests; rules remain in the service |
| `default_root()` / adapters reading environment | Resolve configuration in the composition root |
| `Space::create_dir`, `Project::create_dir`, `exists` | Use cases and filesystem adapters; models perform no I/O |
| `ws.config()` / `ws.save()` | `ws.snapshot()` / automatic persistence in mutating use cases |

The CLI still reads/writes `.spacer/config.json` in its existing format. The old
Git backend's `.spacer/state.json` and directory-scanning registration require explicit reconciliation. See [recovery](../how-to/recover-workspace.md). Those files are left untouched; library callers must explicitly
register projects through the service. There is no recursive project deletion in
the new ports or service.

Invalid configuration now fails when opening any workspace command, including
`workspace init`; initialization never overwrites malformed data. The TUI loads
before entering raw mode, and shell integration still requires no workspace.
