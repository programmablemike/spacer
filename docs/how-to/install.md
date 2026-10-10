# Install from source

[Documentation](../README.md) / How-to guide

## With Cargo

Install a Rust toolchain that provides Cargo, then run these commands from a
checkout of the repository:

```sh
cargo install --path . --locked
spacer --help
```

Add Cargo's binary directory (usually `~/.cargo/bin`) to PATH if your shell cannot
find `spacer`. This installs the CLI and TUI as one binary. The TUI requires an
interactive terminal. Git is needed to run the library's Git integration tests.

## With Nix

With Nix and flakes enabled, run from the repository checkout:

```sh
nix build
./result/bin/spacer --help
```

The built executable is available through `result/bin/spacer`.

## Build for development

With Rust and Git available, run:

```sh
cargo build --locked
cargo test --workspace --locked
```

Run the development binary as `./target/debug/spacer`. The repository also
provides [Nix](../../flake.nix) and [Flox](../../.flox/env/manifest.toml)
configurations. Formatting and lint checks additionally require rustfmt and Clippy.

Continue with [your first workspace](../tutorials/first-workspace.md).
