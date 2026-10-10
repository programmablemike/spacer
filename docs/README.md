# Spacer documentation

Spacer organizes local development projects into spaces and provides a CLI and
read-only terminal browser. The current application tracks directories and
changes in JSON; it does not yet create Git repositories or worktrees.

The documentation follows [Diátaxis](https://diataxis.fr/): choose the kind of help
you need below. Commands describe the implementation in this repository.

| Need | Start here |
| --- | --- |
| Learn through a guided exercise | [Your first workspace](tutorials/first-workspace.md) |
| Install Spacer | [Install from source](how-to/install.md) |
| Navigate projects from your shell | [Set up navigation](how-to/shell-navigation.md) |
| Track a directory that already exists | [Add an existing project](how-to/existing-project.md) |
| Look up commands | [CLI reference](reference/cli.md) |
| Look up state, defaults, and constraints | [Configuration reference](reference/configuration.md) |
| Understand the design | [Workspaces and adapters](explanation/architecture.md) |

For a compact machine-readable documentation index, see [llms.txt](../llms.txt).

## Contributing documentation

Tutorials guide a learner through a complete, repeatable exercise. How-to guides
solve a specific task and assume basic familiarity. Reference describes precise
behavior. Explanation discusses concepts and design choices.

Keep examples aligned with implemented commands and describe observable outcomes.
Use relative links between pages, link back to this index, and update `llms.txt`
when adding or moving documentation. Verify examples in an isolated workspace.
