# Add an existing project

[Documentation](../README.md) / How-to guide

Prerequisites: an initialized workspace and an existing directory to track.
This operation records a directory; it does not clone a repository or validate
Git metadata.

Select the workspace, create a grouping, and record the project's absolute path:

```sh
export SPACER_ROOT="$HOME/code"
spacer space create work
spacer project create api --space work --path /absolute/path/to/api
spacer project list --space work
```

Skip space creation if `work` already exists. The project list shows the custom
path. Existing files remain where they are. If the path does not exist, Spacer
creates a directory there, so check the spelling first.

To select the project for navigation, set both pieces of context:

```sh
spacer space use work
spacer project use api --space work
spacer project go
```

The last command prints the directory, or enters it if
[shell integration](shell-navigation.md) is loaded. Git operations remain under
your control; run Git in that directory as usual.

To stop tracking the project, first finish its tracked changes, then run:

```sh
spacer project delete api --space work
```

The CLI removes the project record and preserves its files. Deletion does not
clean up child records or active selections. Select a remaining valid project
explicitly afterward; there is currently no context-clear command. See
[configuration limitations](../reference/configuration.md).
