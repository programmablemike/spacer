# Spacer

![Spacer logo](logo.svg)

Spacer is a command-line tool and TUI for managing multiple code projects. It's
written in Rust to be blazing fast.

## Table of Contents

- [Description](#description)
- [Use Cases](#use-cases)
  - [Initializing Spacer](#initializing-spacer)
  - [Creating a new project](#creating-a-new-project)
  - [Starting a new change](#starting-a-new-change)
  - [Navigating between projects and changes](#navigating-between-projects-and-changes)
- [Design](#design)
- [License](#license)

## Description

Spacer is a command-line tool and TUI for managing multiple code projects.

It does this by providing an opinionated layout of your workspace for your code
and by integrating with code storage back ends (Git only right now) to retrieve,
update and save changes.

This tool operates in two separate modes: a command-line interface (CLI) for
scripting and automation, and a terminal user interface (TUI) for interactive use.

The CLI allows users to perform various operations on their projects, such as
creating new projects, switching between projects, and managing changes. The TUI
provides a visual interface for users to navigate through their projects and
changes, making it easier to manage multiple projects simultaneously.

## Use Cases

### Initializing Spacer

Initialize Spacer in the current directory:

```sh
spacer workspace init
```

Or point Spacer at an existing directory without changing into it:

```sh
SPACER_ROOT=/path/to/code spacer workspace init
```

### Creating a new project

Create a space, set it as the active context, then create a project inside it:

```sh
spacer space create my-org
spacer space use my-org

spacer project create my-repo
```

Without an active space, pass `--space` explicitly:

```sh
spacer project create my-repo --space my-org
```

List spaces and projects:

```sh
spacer space list
# * my-org   /path/to/code/my-org

spacer project list
#   my-org   my-repo   /path/to/code/my-org/my-repo
```

### Starting a new change

Start a change (creates a tracking entry for a branch or worktree):

```sh
spacer change start feature-x --project my-repo
```

List active changes:

```sh
spacer change list
#   my-org   my-repo   feature-x
```

When the work is done, finish the change:

```sh
spacer change finish feature-x --project my-repo
```

> **Note:** `finish` removes the change from Spacer's tracking. It does not
> delete the underlying git branch or worktree — that remains your responsibility.

### Navigating between projects and changes

Launch the TUI to browse spaces, projects, and changes interactively:

```sh
spacer
```

| Key | Action |
|---|---|
| `Tab` | Switch between Spaces / Projects / Changes tabs |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `q` / `Ctrl-C` | Quit |

## Design

Spacer organises code into a four-level hierarchy:

```
workspace/                  ← root directory ($SPACER_ROOT)
├── .spacer/
│   └── config.json
├── my-org/                 ← space
│   ├── my-repo/            ← project
│   │   ├── main/           ← change (git worktree)
│   │   └── feature-branch/ ← change (git worktree)
│   └── another-repo/
└── another-org/
    └── ...
```

| Concept | Analogous to | Description |
|---|---|---|
| **Workspace** | Local machine | The root directory where all code lives, set via `$SPACER_ROOT` |
| **Space** | GitHub organisation | A named grouping of related projects |
| **Project** | Git repository | A single codebase living inside a space |
| **Change** | Git worktree | An isolated working directory for a branch or task within a project |

The starting point is initializing a workspace with `spacer workspace init` in
the desired root directory, or by setting the `SPACER_ROOT` environment variable.
From there, spaces, projects, and changes can be created using the CLI or TUI.

## License

MIT licensed. See [LICENSE](LICENSE) for more details.
