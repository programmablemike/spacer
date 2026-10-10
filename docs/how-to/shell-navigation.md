# Set up shell navigation

[Documentation](../README.md) / How-to

Use an installed `spacer` on PATH and an initialized workspace. Without a wrapper,
navigation commands print a validated path. The wrapper changes the parent
shell's current directory only after successful resolution.

## Persistent integration

Add the appropriate line to your shell startup file:

| Shell | File | Line |
| --- | --- | --- |
| zsh | `~/.zshrc` | `eval "$(spacer shell integration zsh)"` |
| bash | `~/.bashrc` | `eval "$(spacer shell integration bash)"` |
| sh | `~/.profile` | `eval "$(spacer shell integration sh)"` |

Export an absolute root there if you want one workspace selected across sessions:

```sh
export SPACER_ROOT="$HOME/code"
```

Otherwise, Spacer finds the nearest ancestor workspace. External project paths
need an explicit root. Start a new shell or source your startup file, then use
registered targets:

```sh
spacer space go personal
spacer project go website --space personal
spacer go personal/website
```

`spacer go personal/website/feat/login` navigates to the actual Git checkout for
the branch `feat/login`. Missing targets return an error and preserve the current
directory. Other commands continue to invoke the binary normally.

## Temporary managed sessions

On macOS/Linux, run `spacer shell` from a workspace and `exit` to leave it. Choose
a shell explicitly with `spacer shell --shell /bin/bash` or `--shell /bin/zsh`.
The session exports an absolute `SPACER_ROOT` and `SPACER_SHELL_ACTIVE=1`; nested
managed sessions are refused.

Bash loads your `.bashrc`, then the navigation wrapper, prompt indicator, and
static completions. Zsh loads your `.zshenv` and `.zshrc`, respecting the original
`ZDOTDIR`. It skips global interactive startup files to keep bootstrap predictable;
the system `.zshenv` still runs. Your own startup files remain responsible for any
interactive prompts they introduce. POSIX sh receives the workspace environment
only; load the wrapper separately if needed.

`spacer completions bash`, `zsh`, or `fish` prints a static completion script.
Fish has no managed-session support in alpha.

## Without a wrapper

Use the path directly while preserving resolution failure:

```sh
project_path=$(spacer path project website --space personal) && cd "$project_path"
```

Windows has no managed shell or supported navigation wrapper. In PowerShell:

```powershell
$projectPath = spacer path project website --space personal
if ($LASTEXITCODE -eq 0) { Set-Location -LiteralPath $projectPath }
```

See [configuration](../reference/configuration.md) for discovery and context rules.
