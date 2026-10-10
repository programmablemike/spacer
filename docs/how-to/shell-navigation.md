# Set up shell navigation

[Documentation](../README.md) / How-to guide

Prerequisites: an installed `spacer` on PATH and an initialized workspace with
at least one space and project.

Set the root to your workspace's absolute path:

```sh
export SPACER_ROOT="$HOME/code"
```

Add the appropriate integration line to your shell's startup file:

| Shell | File | Line |
| --- | --- | --- |
| zsh | `~/.zshrc` | `eval "$(spacer shell zsh)"` |
| bash | `~/.bashrc` | `eval "$(spacer shell bash)"` |
| sh | `~/.profile` | `eval "$(spacer shell sh)"` |

Also export `SPACER_ROOT` there if you want the same workspace in future sessions.
Start a new shell or source the startup file. With `personal` and `website`
already registered, run:

```sh
spacer space go personal
spacer project go website --space personal
pwd
```

The final command prints the project's directory. The wrapper intercepts these
two navigation commands; other commands continue to run the binary normally.
`spacer shell` prints integration code; it does not launch a managed shell.

Without integration, use the binary's printed path explicitly:

```sh
project_path=$(command spacer project go website --space personal) && cd "$project_path"
```

This form also propagates resolution failure before attempting `cd`. The current
wrapper does not explicitly guard that failure; use known registered targets.
Other shells can consume printed paths using their own directory-changing syntax.

If projects disappear after navigation, check `SPACER_ROOT`. Without that
variable, each invocation treats its current directory as the workspace root;
there is no ancestor search. See [configuration](../reference/configuration.md).
