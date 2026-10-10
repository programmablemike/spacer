# Verify a release

[Documentation](../README.md) / How-to

Pull requests and main run the reusable Quality workflow on macOS ARM64/x86-64,
Linux ARM64/x86-64, and Windows x86-64. Each native runner executes formatting,
Clippy, workspace tests, a fresh source installation, and an installed-binary
lifecycle. POSIX runners execute the tutorial commands from the documentation and exercise bash/zsh/sh sessions and TUI restoration
through pseudo-terminals. Runner labels follow the [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

`tests/smoke.py BINARY` runs the fresh-workspace lifecycle against an installed
binary. `scripts/package.py BINARY TARGET` creates a native archive and SHA-256
file in `dist/`, extracts it, and runs the same lifecycle against the extracted
binary. CI uploads these checked archives for review.

Release tags must match all package versions. Publication reruns the complete
Quality workflow for the tagged revision and checks all five archives and their
checksums before uploading. Versions below 1.0 are marked prerelease. Release
notes live in `RELEASE_NOTES.md`; GitHub publication is driven by version tags.

For local verification, use the pinned Rust toolchain or `nix develop`, with Git,
Python 3.9+, bash, and zsh installed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
python3 tests/smoke.py target/release/spacer
python3 tests/tutorial.py target/release/spacer
python3 tests/terminal.py target/release/spacer
```

Windows uses `target/release/spacer.exe` and omits the POSIX tutorial and terminal tests. CI uses native archives built by
`scripts/package.py`; the former cargo-dist configuration is no longer used.


## Desktop gates

The separate Desktop workflow runs TypeScript checks, formatting, Chromium/WebKit
UI tests, native Rust checks, an actual webview lifecycle, and installer builds. It uses the same five native
targets as the CLI. Version-tag publication depends on both workflows and requires
CLI archives plus DMG/Debian/NSIS desktop installers, with matching versions and
checksums. UI test reports are not release assets.

From `desktop/`, run `npm ci`, `npm run format:check`, `npm run build`,
`npx playwright install chromium webkit`, and `npm test`. Native checks use
`cargo test --manifest-path src-tauri/Cargo.toml --locked` and the analogous
formatting/Clippy commands. See [desktop setup](desktop.md) for system prerequisites
and the installer build command.

## Nix packages

The Nix workflow runs `nix flake check` on ARM64 and x86-64 macOS/Linux. It builds
the CLI, frontend, and desktop from source, runs Rust tests, and exercises the
packaged CLI and a test-only native webview. The webview starts with an empty
PATH to verify that the Nix wrapper supplies Git. Package layout checks cover
the macOS application bundle and Linux desktop entry. Version-tag publication
also depends on this workflow.

See [Nix installation](nix.md) for package outputs and Home Manager/nix-darwin
integration. Nix store outputs are separate from standalone release installers.
