# Repository guidance

## Working on changes

- Read applicable instructions and relevant configuration before making changes.
- Do implementation and documentation work in a dedicated Git worktree on a
  task-specific branch. Keep the main checkout available for unrelated work.
- Submit changes through a pull request. Do not commit directly to the default
  branch or merge changes without authorization.
- Preserve unrelated staged, unstaged, and untracked files. Stage only the files
  belonging to the task; review the staged diff before committing.
- Keep changes focused and follow the existing architecture and dependencies.
- Never include credentials, tokens, private keys, or other secrets in source,
  logs, prompts, or generated artifacts.

## Project structure and verification

- This is a Rust workspace. `src/cli/` and `src/tui/` provide presentation;
  `spacer-core/` contains domain models, application services, and ports;
  `spacer-data/` implements JSON, filesystem, and Git adapters. `desktop/`
  contains a Tauri presentation and a separate native Cargo workspace.
- Keep dependencies pointing inward. Core code must not import data adapters,
  presentation frameworks, or perform direct I/O. Construct adapters in the
  executable composition root, and keep use-case policy in the core.
- Presentation calls application services and renders domain results. Data
  adapters implement core-owned ports without deciding lifecycle semantics.
- Use the development tooling defined by `flake.nix` or `.flox/` when needed.
- For Rust changes, run focused verification and the applicable workspace checks:
  `cargo test --workspace --locked`, `cargo fmt --all -- --check`, and
  `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- For desktop changes, run `npm ci`, `npm run format:check`, `npm run build`,
  and `npm test` from `desktop/` (install Playwright browsers first). Run Rust
  formatting, Clippy, and tests with `--manifest-path desktop/src-tauri/Cargo.toml`,
  and build the native bundle. Keep GUI dependencies out of the CLI workspace.
- Git integration tests require Git on PATH and must use isolated temporary
  repositories. Never exercise destructive behavior against a user's projects.
- For documentation-only changes, check the diff, links, and factual consistency;
  a full Rust test run is not required.
- PR descriptions should explain the problem, resulting behavior, verification,
  and any remaining limitations. Report unavailable checks rather than claiming
  they passed.

## Releases and versioning

Releases use `MAJOR.MINOR.PATCH`. Spacer uses the following pre-1.0 maturity
convention, followed by semantic versioning for stable releases:

| Version | Meaning |
| --- | --- |
| `0.0.1` | Initial alpha; the `0.0.x` series is alpha. |
| `0.1.0` | Initial beta; the `0.x.y` series with a nonzero minor is beta. |
| `1.0.0` | Initial stable release. |
| `1.0.1` | Stable patch release with backward-compatible fixes. |
| `1.1.0` | Stable minor release with backward-compatible functionality. |
| `2.0.0` | Stable major release with backward-incompatible changes. |

- After stability, increment patch for backward-compatible fixes, minor for
  backward-compatible features, and major for backward-incompatible changes.
  Reset lower components to zero when incrementing major or minor.
- Treat CLI behavior, configuration formats, documented workflows, and the
  published library API as compatibility surfaces. Document breaking changes and
  migration steps, including during alpha and beta.
- Keep release versions consistent across package manifests, lockfile package
  entries, and packaging metadata where applicable. Do not change versions as an
  incidental part of unrelated work.
- Prepare releases through PRs, record validation and release notes, and publish
  only when authorized. Mark alpha and beta GitHub releases as prereleases.
