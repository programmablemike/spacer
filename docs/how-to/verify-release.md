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
