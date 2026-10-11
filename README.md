<p align="center">
  <img src="logo.svg" alt="Spacer — a grid of blocks with one yellow block docking into place" width="640">
</p>

# Spacer

[Documentation](docs/README.md) · [Documentation index for LLMs](llms.txt)

Organize and navigate your local development projects from the command line.
Spacer groups projects into spaces and provides a terminal browser for your
workspace. A Tauri desktop app provides the same project and worktree lifecycle
through a graphical interface.

**Status: 0.0.1 alpha.** Spacer creates and registers local Git repositories
and manages linked worktrees. Finish preserves branches; unregister preserves
files. See [recovery guidance](docs/how-to/recover-workspace.md) for legacy data.

## Desktop app

Open or create a workspace, browse projects, start and finish worktrees, and open
folders graphically. The desktop reuses the CLI's application services and file
preservation rules. See the [desktop guide](docs/how-to/desktop.md) for source
setup and native builds. The existing 0.0.1 release contains CLI/TUI binaries;
desktop installers are available from the Desktop workflow and future releases.

```sh
cd desktop
npm ci
npm run tauri -- dev
```

## GitHub setup and commit identities

Choose **Set up from GitHub** in the desktop to connect an account, select
repositories, choose commit authorship, and review destinations before cloning.
The CLI provides the same services through `spacer connector`, `spacer identity`,
and `spacer setup`. Managed credentials use the system credential manager.

See [GitHub setup](docs/how-to/github-setup.md) for authorization prerequisites,
existing credentials, scripting, and recovery.

## Install the CLI

Download a checksummed archive from the [alpha release](https://github.com/programmablemike/spacer/releases/tag/v0.0.1),
or build from source with the pinned Rust 1.94.0 toolchain. Git must be on PATH.
See [installation](docs/how-to/install.md) for platform selection and checksums.
From a checkout of this repository:

```sh
cargo install --path . --locked
spacer --help
```

Ensure Cargo's installation directory (usually `~/.cargo/bin`) is on your `PATH`.
Configure your own Git commit identity before making the tutorial's first commit.

With Nix and flakes enabled, build either presentation from the checkout:

```sh
nix build .#cli
./result/bin/spacer --help
nix build .#desktop --out-link result-desktop
```

`default` remains an alias for `cli`. See [Nix installation](docs/how-to/nix.md)
for Home Manager, nix-darwin, application registration, and updates.

## Quickstart

The examples below use a POSIX-compatible shell. Choose a workspace directory and
export its absolute path so commands continue to find it after you change
folders:

```sh
mkdir -p "$HOME/code"
export SPACER_ROOT="$HOME/code"
unset SPACER_SPACE SPACER_PROJECT
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

`project create` initializes a new Git repository without making a commit.
Register an existing primary repository without moving it:

```sh
spacer project register existing-repo --space personal --path /absolute/path/to/repo
```

## Navigate from your shell

Without shell integration, `space go` and `project go` print directory paths.
Load the wrapper to make them change your shell's current directory:

| Shell | Add to your startup file |
| --- | --- |
| zsh (`~/.zshrc`) | `eval "$(spacer shell integration zsh)"` |
| bash (`~/.bashrc`) | `eval "$(spacer shell integration bash)"` |
| sh (`~/.profile`) | `eval "$(spacer shell integration sh)"` |

Restart your shell or source the appropriate file, then run:

```sh
spacer space go personal
spacer project go website
```

Export `SPACER_ROOT` in your shell startup file as well if you want it to persist
across sessions. Without it, Spacer searches for the nearest ancestor workspace. External project
paths require an explicit root. For a temporary managed session, run `spacer shell`.

## Work on a change

A change is a linked Git worktree. Make an initial commit before starting one:

```sh
git -C "$SPACER_ROOT/personal/website" commit --allow-empty -m "Initial commit"
spacer change start feature-x --project website --space personal
spacer change list --project website --space personal
spacer go personal/website/feature-x
cd "$SPACER_ROOT"
spacer change finish feature-x --project website --space personal
```

`start` creates a branch and checkout under `.spacer/worktrees`. `finish` removes
only a clean linked checkout and retains its branch and commits. Dirty, ignored,
locked, and primary checkouts are refused. Legacy tracking entries are available
through `change list --tracked` and `change finish NAME --project PROJECT --tracked`.

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
| Project | Registered primary Git repository within a space |
| Change | Linked Git worktree within a project |
| Connector | Authenticated access to a GitHub account |
| Identity | Explicit Git commit name and email, independent of remote access |

Commands use explicit names and flags when provided. Otherwise, supported
commands use environment overrides followed by saved defaults:

| Variable | Purpose |
| --- | --- |
| `SPACER_ROOT` | Explicit workspace root; otherwise nearest ancestor workspace |
| `SPACER_SPACE` | Overrides the saved active space when nonempty |
| `SPACER_PROJECT` | Overrides the saved active project when nonempty |

Use `spacer space current` and `spacer project current` to inspect selections.
`change start` and `change finish` still require `--project` explicitly.
For command details, run `spacer --help` or a subcommand's `--help`.

## Current limitations

- Setup supports cloning existing GitHub.com repositories over HTTPS. Fetch, push,
  merge, branch deletion, and remote repository creation remain user-managed.
- `space delete` and `project delete` remove tracking records, not directories.
  Spaces must have no registered projects. Projects must have no legacy tracking
  entries; their Git worktrees can remain. Affected saved selections are cleared.
- Configuration writes are atomic and detect concurrent changes. Retry a
  conflicting command after reopening the workspace; back up metadata before
  manual edits.
- Shell integration is provided for bash, zsh, and sh. Other shells can use the
  paths printed by navigation commands with their own directory-changing syntax.
  Managed shells are macOS/Linux only.

See [release notes](RELEASE_NOTES.md) for tested platforms, migration, and alpha
limits, including partial-failure recovery and session context validation.

## Development

The Rust workspace separates presentation (`src/cli/`, `src/tui/`), the domain
model and application services (`spacer-core/`), and data adapters (`spacer-data/`).
The executable composition root connects them. See the
[hexagonal architecture](docs/explanation/architecture.md) and
[Rust API reference](docs/reference/rust-api.md). Nix and Flox development configurations are
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
