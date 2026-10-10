# Use the desktop app

[Documentation](../README.md) / How-to

Spacer's Tauri app provides graphical access to the same workspace registry and
Git lifecycle as the CLI. It runs locally and does not require a Spacer server
or a separately installed Spacer CLI. Git must be on PATH.

The desktop is available from source and Desktop CI artifacts. The published
0.0.1 release predates the desktop app and contains CLI/TUI binaries only. Future
version-tag releases include checksummed desktop installers after native checks.

## Run from source

Install Node.js 22.12+ (or a compatible newer LTS version), the repository's Rust
1.94.0 toolchain, Git, and the [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/).
macOS needs Xcode command-line tools. Windows needs the MSVC build tools and
WebView2. Linux needs WebKitGTK 4.1 and its development dependencies.

From the repository root:

```sh
cd desktop
npm ci
npm run tauri -- dev
```

`npm run dev` alone serves the frontend without the native workspace bridge; use
`tauri dev` for a working application. Cargo and rustc must both be on PATH.
The repository's Nix environment is for the CLI; native desktop bundling uses
Tauri's platform tooling.

## Choose a workspace

Use **Browse** to select a folder, then **Open workspace**. For a new workspace,
choose **Create workspace** instead. Initialization adds `.spacer/config.json`
and preserves existing files. Opening an uninitialized folder never initializes
it implicitly. Recent workspace paths are saved only in the desktop's local UI
storage; the app does not automatically open them on launch.

The desktop uses its explicit folder selection. It does not consume
`SPACER_ROOT`, `SPACER_SPACE`, or `SPACER_PROJECT` from a launching shell.

## Organize projects

Create a space using the plus button beside **Spaces**. Select **New project** to
initialize a Git repository in a new or empty folder, or **Register project** to
add an existing primary repository without moving it. A custom folder must be an
absolute path; leaving it empty uses the normal workspace layout.

Selecting a space or project changes the desktop view only. **Use as default**
saves that selection for the CLI and other sessions. **Unregister** removes only
the registration and preserves all files, branches, and worktrees. Parent and
legacy-tracking guards still apply.

## Work on a change

Open a project and choose **Start change**. Supply a new branch name, including
slashes if desired. Git must have an initial commit; make that commit using your
usual Git tools and your own identity. Spacer does not create commits for you.

**Open** opens the actual directory in Finder, Explorer, or the default file
manager. **Copy path** validates the target again before copying it. Detached
worktrees can be opened and copied but must be managed directly with Git.

**Finish** displays the target for review, then removes only a clean linked
checkout. The branch and all commits remain. Dirty, untracked, ignored, locked,
and primary checkouts are refused. Failed actions preserve the dialog inputs
and show the service's error so you can correct the problem and retry.

Legacy tracking entries appear separately and can be removed without Git effects.

## Refresh and recover

Use **Refresh** to observe changes made with the CLI or Git. Each native operation
reloads the registry; in-process requests are serialized off the UI thread and
registry writes retain the shared conflict checks. There is no filesystem watcher
or automatic merge of edits. External changes during a mutation can still cause
a conflict or partial failure; see [recovery](recover-workspace.md).

A missing or unreadable repository stays visible with its error. It does not
prevent working with other projects or unregistering the broken registration.
An unreadable workspace registry is an error and is never overwritten by opening
or initializing it.

## Build an installer

From `desktop/`:

```sh
npm ci
npm run tauri -- build --ci -- --locked
```

Installers are written below `desktop/src-tauri/target/release/bundle/` (or your
`CARGO_TARGET_DIR`). macOS builds an app/DMG, Linux a Debian package, and Windows
an NSIS installer. The Desktop workflow checks all five native targets used by
the CLI, then uploads separately named desktop installers and SHA-256 files.
Builds are unsigned; signing, notarization, automatic updates, and mobile apps
are not configured.

Desktop browser tests use a mocked IPC boundary to exercise forms, error handling,
navigation, and layout in Chromium and WebKit. Rust bridge tests use actual Git
and isolated temporary workspaces. A separate `native-smoke` build drives the real webview through initialization,
registration, start, finish, and unregister, then verifies branch/file preservation.
This harness uses an isolated temporary workspace and a nonpersistent webview;
it is excluded from normal builds. Native CI compiles and bundles each target.
It does not automate each operating system's installer or file-manager UI.
