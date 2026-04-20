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

@TODO: Add examples of the different ways Spacer can be initialized.

### Creating a new project

@TODO: Add examples of creating a new Spacer space and project using both the CLI and TUI.

### Starting a new change

@TODO: Add examples that show the full lifecycle of a change starting from creation to code updates and finally saving the change.

### Navigating between projects and changes

@TODO: Add examples that show how you can quickly navigate between projects and changes using both the CLI and TUI.

## Design

We introduce introduces the following concepts:

- Space: A collection of projects. This is useful for grouping related projects
together for cleanliness.
- Project: Directory containing the main code.
- Change: A change is a set of files that are being work on. This is useful for
keeping track of what files are being worked on and for easily switching between
 different changes.  

The starting point for creating a new Spacer space is initializing a Spacer root
directory where the code will be stored. This can be set via the environment
variable `SPACER_ROOT` or by running `spacer init` in the desired directory.

Once the root directory is set, you can create new spaces and projects within its
This can be done using the CLI or TUI; we recommend the TUI for anyone new to
this tool.

## License

MIT licensed. See [LICENSE](LICENSE) for more details.
