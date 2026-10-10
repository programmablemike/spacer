# Register an existing project

[Documentation](../README.md) / How-to

Use an initialized workspace and an existing primary non-bare Git repository.
Registration validates the repository without moving, initializing, or committing
its files. Linked checkout paths and bare repositories are rejected.

```sh
export SPACER_ROOT="$HOME/code"
spacer space create work
spacer project register api --space work --path /absolute/path/to/api
spacer project use api --space work
spacer project go
```

Skip space creation if it already exists. Selecting the project also selects its
space. The final command prints its path or navigates through shell integration.

```sh
spacer project unregister api --space work
```

Unregister preserves all files, Git records, branches, and worktrees. Registering
again discovers the worktrees at their actual Git-reported paths. Legacy metadata
tracking entries must first be removed explicitly with `change finish --tracked`.
`project delete` is a deprecated alias for unregister. See [recovery](recover-workspace.md).
