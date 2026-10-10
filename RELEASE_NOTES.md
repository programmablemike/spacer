# Spacer 0.0.1 — alpha

Spacer organizes local Git repositories into spaces and manages linked worktrees
from a CLI, with shell navigation and a read-only terminal browser.

## Included workflows

- Create a Git project or register an existing primary repository without moving it.
- Start a branch and linked checkout, navigate using its actual Git path, and finish
  a clean checkout while retaining its branch and commits.
- Discover the nearest workspace or select one explicitly with `SPACER_ROOT`.
- Use bash/zsh managed sessions, bash/zsh/sh navigation wrappers, and static completions.
- Persist registrations and context through atomic, conflict-detecting writes.
  Unregister operations preserve project files and Git worktrees.

## Platforms and verification

| Native target | CI environment | Verification |
| --- | --- | --- |
| macOS ARM64 | macOS 15 | Rust checks, installed/archive lifecycle, shell and TUI PTY checks |
| macOS x86-64 | macOS 15 Intel | Same |
| Linux ARM64 | Ubuntu 24.04 | Same |
| Linux x86-64 | Ubuntu 24.04 | Same |
| Windows x86-64 MSVC | GitHub Windows runner | Rust checks, installed/archive CLI lifecycle, renderer tests |

Each release archive has a SHA-256 file. Publication requires passing checks on
the tagged revision. The workflow logs record OS, Rust, Git, and shell versions;
Rust is pinned to 1.94.0. macOS terminal tests explicitly exercise bundled Bash
3.2 as well as zsh. Windows interactive TUI behavior has no automated PTY coverage.

See [installation](https://github.com/programmablemike/spacer/blob/v0.0.1/docs/how-to/install.md), the
[tutorial](https://github.com/programmablemike/spacer/blob/v0.0.1/docs/tutorials/first-workspace.md), and
[release verification](https://github.com/programmablemike/spacer/blob/v0.0.1/docs/how-to/verify-release.md).

## Compatibility and recovery

The initial alpha uses version 0.0.1; the previous 0.1.0 package metadata did not
represent a beta release. The libraries now use separate Registry, Filesystem,
and Git ports with the Workspace application service. The previous Backend/store
API is removed; see [API migration](https://github.com/programmablemike/spacer/blob/v0.0.1/docs/reference/rust-api.md).

The CLI now performs real Git worktree operations. Legacy metadata-only changes
remain accessible through `--tracked`. `project delete` and `space delete` are
deprecated aliases for unregistering and preserve files.

Valid unversioned configuration with absolute paths loads read-only and upgrades
to schema 1 on the next successful mutation. Unknown fields/schemas, relative
legacy paths, or former `.spacer/state.json` require explicit reconciliation.
Back up metadata and follow [recovery guidance](https://github.com/programmablemike/spacer/blob/v0.0.1/docs/how-to/recover-workspace.md).

## Alpha limitations

- APIs, CLI behavior, and configuration may change before stability.
- The TUI is a read-only snapshot. There is no graphical desktop application.
- Clone, fetch, push, merge, branch deletion, and recursive directory deletion
  remain user-managed Git/filesystem operations.
- Git/filesystem effects and registry writes are not one transaction. A failed
  save can leave a directory or repository to inspect and recover. Avoid external
  filesystem changes while a mutation is running.
- Unknown session context overrides fail during workspace loading, including
  commands with explicit flags; correct or unset them before retrying.
- Managed shells are macOS/Linux only; sh sessions receive environment only.
  Fish supplies completion output, not managed sessions. Windows uses explicit
  path output with its parent shell.
- Older operating systems and musl Linux are unverified. Binaries are not signed
  or notarized.
