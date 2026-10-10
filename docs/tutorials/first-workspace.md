# Your first workspace

[Documentation](../README.md) / Tutorial

In this exercise you will create a workspace, organize a project, and track a
piece of work. You need an [installed Spacer binary](../how-to/install.md) on PATH
and a POSIX-compatible shell. No existing repositories are needed.

## Create a practice workspace

Use a new temporary directory so the exercise does not change your projects:

```sh
export SPACER_ROOT="$(mktemp -d)"
unset SPACER_SPACE SPACER_PROJECT
spacer workspace init
```

Spacer prints the initialized path and creates `.spacer/config.json` there.
Keep using this shell for the rest of the exercise.

## Organize a project

```sh
spacer space create personal
spacer space use personal
spacer project create website
spacer project use website
spacer project list
```

The list contains `website`, marked as active, under the `personal` space.
The filesystem now contains:

```text
<temporary directory>/
├── .spacer/config.json
└── personal/website/
```

The project directory is empty; Spacer has not initialized a Git repository.

## Find your project

```sh
spacer project go website
```

This prints the project's path. Without a shell wrapper it does not change the
current directory. Follow the [navigation guide](../how-to/shell-navigation.md)
when you want directory-changing integration.

## Track a change

```sh
spacer change start update-homepage --project website
spacer change list --project website
spacer change finish update-homepage --project website
spacer change list --project website
```

The first list contains `update-homepage`; the second reports no changes.
These commands add and remove a tracking record. They do not create a branch,
create a checkout, or modify project files.

## Browse the result

In an interactive terminal, run:

```sh
spacer
```

Press `Tab` to move between spaces, projects, and changes. Use arrow keys to move
within a list and `q` to quit. The project remains listed; its finished change
is absent. The browser is read-only.

You now have a workspace with one space and project. Its path remains in
`SPACER_ROOT`; keep it for experimentation or remove that temporary directory
with your usual filesystem tools after checking its contents. Start with
[adding an existing project](../how-to/existing-project.md) for your own code.
