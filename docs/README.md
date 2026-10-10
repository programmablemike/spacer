# Spacer documentation

Spacer organizes local Git projects into spaces, manages linked worktrees, and
provides a Tauri desktop app, shell navigation, and a read-only terminal browser.

The documentation follows [Diátaxis](https://diataxis.fr/): choose the kind of help
you need below.

| Need | Start here |
| --- | --- |
| Learn through a guided exercise | [Your first workspace](tutorials/first-workspace.md) |
| Use the graphical app | [Desktop guide](how-to/desktop.md) |
| Install Spacer | [Installation](how-to/install.md) |
| Navigate from your shell | [Set up navigation](how-to/shell-navigation.md) |
| Register a repository you already have | [Add an existing project](how-to/existing-project.md) |
| Recover after an error or upgrade | [Workspace recovery](how-to/recover-workspace.md) |
| Verify a build or release | [Release verification](how-to/verify-release.md) |
| Look up commands | [CLI reference](reference/cli.md) |
| Look up state, defaults, and constraints | [Configuration reference](reference/configuration.md) |
| Integrate with the Rust libraries | [Rust API and migration](reference/rust-api.md) |
| Understand the design | [Hexagonal architecture](explanation/architecture.md) |

See [release notes](../RELEASE_NOTES.md) for supported platforms and alpha limits,
and [llms.txt](../llms.txt) for a compact documentation index.

## Contributing documentation

Tutorials guide a learner through a complete exercise. How-to guides solve a
specific task. Reference describes precise behavior. Explanation discusses
concepts and design choices.

Keep examples aligned with implemented commands. Use relative links between
pages, link back to this index, and update `llms.txt` when moving or adding pages.
Verify examples in an isolated workspace.
