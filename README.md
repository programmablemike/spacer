<p align="center">
  <img src="logo.svg" alt="Spacer — a grid of blocks with one yellow block docking into place" width="640">
</p>

# Spacer

[Documentation](docs/README.md) · [Documentation index for LLMs](llms.txt)

Organize and navigate your local development projects from the command line.
Spacer groups projects into spaces and provides a terminal browser for your
workspace.

**Status: early development.** The CLI currently creates directories and tracks
spaces, projects, and changes in JSON. Git repository and worktree operations
exist in the core library but are not connected to the CLI or TUI yet.

## Install from source

You need a Rust toolchain with Cargo. From a checkout of this repository:

```sh
cargo install --path . --locked
spacer --help
```

Ensure Cargo's installation directory (usually `~/.cargo/bin`) is on your `PATH`.
Git is needed for the core library's Git adapter and its integration tests.

With Nix and flakes enabled, you can build and run from the checkout instead:

```sh
nix build
./result/bin/spacer --help
```

## Quickstart

The examples below use a POSIX-compatible shell. Choose a workspace directory and
export its absolute path so commands continue to find it after you change
folders:

```sh
mkdir -p "$HOME/code"
export SPACER_ROOT="$HOME/code"
spacer workspace init

spacer space create personal
spacer space use personal
spacer project create website
spacer project use website

spacer space list
spacer project list
```

This creates the following directories and saves the active space/project in
`.spacer/config.json`:

```text
code/
├── .spacer/
│   └── config.json
└── personal/
    └── website/
```

`project create` creates a directory, not a Git repository. Initialize or clone
repositories with Git separately.

To track an existing project directory without moving it:

```sh
spacer project create existing-repo --space personal --path /absolute/path/to/repo
```

The path is recorded and the directory is created if missing. This command does
not check whether the directory is a Git repository. Prefer absolute custom paths.

## Navigate from your shell

Without shell integration, `space go` and `project go` print directory paths.
Load the wrapper to make them change your shell's current directory:

| Shell | Add to your startup file |
| --- | --- |
| zsh (`~/.zshrc`) | `eval "$(spacer shell zsh)"` |
| bash (`~/.bashrc`) | `eval "$(spacer shell bash)"` |
| sh (`~/.profile`) | `eval "$(spacer shell sh)"` |

Restart your shell or source the appropriate file, then run:

```sh
spacer space go personal
spacer project go website
```

Export `SPACER_ROOT` in your shell startup file as well if you want it to persist
across sessions. Without it, Spacer uses the current directory as the workspace
root; it does not search parent directories.

## Track changes

A change is currently a named tracking entry within a project:

```sh
spacer change start feature-x --project website --space personal
spacer change list --project website --space personal
spacer change finish feature-x --project website --space personal
```

`start` adds an entry to the workspace configuration. It does not create a branch
or worktree. `finish` removes that entry and leaves Git branches and files alone.
Use Git directly to manage the underlying work for now.

## Browse the workspace

Run `spacer` without arguments to open the read-only TUI:

```sh
spacer
```

| Key | Action |
| --- | --- |
| `Tab` | Switch between spaces, projects, and changes |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `q` / `Ctrl-C` | Quit |

The TUI displays a snapshot. Restart it to see changes made by other commands.
It does not create objects or change the parent shell's directory.

## Concepts and context

| Concept | Current meaning |
| --- | --- |
| Workspace | Root directory containing `.spacer/config.json` |
| Space | Named group of projects, usually an organization or personal collection |
| Project | Named directory within a space, optionally at a custom path |
| Change | Named record of work within a project |

Commands use explicit names and flags when provided. Otherwise, supported
commands use environment overrides followed by saved defaults:

| Variable | Purpose |
| --- | --- |
| `SPACER_ROOT` | Workspace root; defaults to the current directory |
| `SPACER_SPACE` | Overrides the saved active space when nonempty |
| `SPACER_PROJECT` | Overrides the saved active project when nonempty |

Use `spacer space current` and `spacer project current` to inspect selections.
`change start` and `change finish` still require `--project` explicitly.
For command details, run `spacer --help` or a subcommand's `--help`.

## Current limitations

- The CLI tracks metadata; Git automation is not integrated yet.
- `space delete` and `project delete` remove tracking records, not directories.
  They do not clean up child records or saved active selections. Finish tracked
  changes before deleting a project, remove projects before deleting a space,
  and select a remaining valid context afterward.
- Configuration writes are not atomic or coordinated between processes. Avoid
  simultaneous commands that modify the same workspace and back up
  `.spacer/config.json` before significant changes.
- Shell integration is provided for bash, zsh, and sh. Other shells can use the
  paths printed by navigation commands with their own directory-changing syntax.

## Development

The Rust workspace contains the CLI/TUI in `src/` and domain types, storage, and
Git adapters in `spacer-core/`. Nix and Flox development configurations are
included in [flake.nix](flake.nix) and [.flox/env/manifest.toml](.flox/env/manifest.toml).

With Rust, rustfmt, Clippy, and Git available:

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The visual identity represents organized project spaces: a four-by-four grid
with one yellow block docking into place. [logo.svg](logo.svg) is the wordmark;
[logo-icon.svg](logo-icon.svg) is the standalone icon. Both are self-contained SVGs.

## License

[MIT](LICENSE).
