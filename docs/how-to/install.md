# Install Spacer

[Documentation](../README.md) / How-to

Spacer requires Git on PATH for repository and worktree operations. Use a recent
Git 2.x release; the release workflow records the exact tested version. Configure
your own Git commit identity before the tutorial's first commit. Spacer neither
configures your identity nor makes commits for you.

## From a release archive

Download the archive and matching `.sha256` file from the
[0.0.1 alpha release](https://github.com/programmablemike/spacer/releases/tag/v0.0.1).
Choose the target for your platform:

| Platform | Target | Archive |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |
| Linux ARM64, glibc | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| Linux x86-64, glibc | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `.zip` |

The native release tests use macOS 15, Ubuntu 24.04, and GitHub's Windows runner.
Older OS versions, musl Linux, and other architectures are unverified.

On macOS, for example, verify and extract the Apple Silicon archive from your
download directory:

```sh
shasum -a 256 -c spacer-0.0.1-aarch64-apple-darwin.tar.gz.sha256
tar -xzf spacer-0.0.1-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
cp spacer-0.0.1-aarch64-apple-darwin/spacer "$HOME/.local/bin/spacer"
export PATH="$HOME/.local/bin:$PATH"
spacer --version
```

On Linux, use your target filename and `sha256sum -c` for verification. Add the
PATH export to your shell startup file to keep it in future sessions.

On Windows, compare the hash and extract in PowerShell:

```powershell
$archive = 'spacer-0.0.1-x86_64-pc-windows-msvc.zip'
$expected = ((Get-Content "$archive.sha256") -split '\s+')[0]
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne $expected) { throw 'Checksum mismatch' }
Expand-Archive $archive -DestinationPath .
.\spacer-0.0.1-x86_64-pc-windows-msvc\spacer.exe --version
```

Place `spacer.exe` in a directory on your user PATH. Release binaries are not
code-signed or notarized; checksums detect download corruption and do not provide
an independent authenticity signature.

## From source with Cargo

Use the Rust 1.94.0 toolchain pinned by `rust-toolchain.toml`, with your platform's
linker/build tools. From a checkout of tag `v0.0.1`:

```sh
cargo install --path . --locked
spacer --help
```

Add Cargo's binary directory (usually `~/.cargo/bin`) to PATH. The CLI and TUI
share one executable. The TUI requires an interactive terminal.

## With Nix

With Nix and flakes enabled, run from the repository checkout:

```sh
nix build
./result/bin/spacer --help
nix build .#desktop --out-link result-desktop
```

The `cli` and `desktop` packages include their Git runtime dependency. See
[Nix installation](nix.md) for Home Manager, nix-darwin, application registration,
and version pinning. The default package remains the CLI.

For development, `nix develop` or the [Flox environment](../../.flox/env/manifest.toml)
provides tooling. See [release verification](verify-release.md) for checks.
Continue with [your first workspace](../tutorials/first-workspace.md).
