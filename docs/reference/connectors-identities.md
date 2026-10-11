# Connectors and commit identities

[Documentation](../README.md) / Reference

## User catalog

Connector and identity metadata lives in `accounts.json` in the user configuration
directory. On Unix the default is `$XDG_CONFIG_HOME/spacer`, falling back to
`$HOME/.config/spacer`; on Windows it is `%APPDATA%/spacer`. An absolute
`SPACER_CONFIG_DIR` overrides the directory for both presentations. The catalog uses
schema version 1, atomic replacement, an advisory writer lock, and optimistic
conflict detection.

Identities contain a stable ID, unique identity label, name, and email. Connectors
contain a stable ID, unique connector label, host, stable remote account ID, login,
and credential source (`keychain` or `github-cli`). Identity and connector labels
are separate namespaces. CLI selections accept either a stable ID or its label.
Only `github.com` is currently supported. Managed login uses an OAuth App device
flow with `repo offline_access`. Repository discovery paginates the authenticated
user's `/user/repos` endpoint and does not require repository installation.
Organization approval and SSO restrictions still apply. The `repo` grant includes
write access within the user's permissions, even though setup only reads/clones.

The public OAuth App client ID is bundled. `SPACER_GITHUB_CLIENT_ID` overrides it
for CLI and desktop; `connector login --client-id` overrides it for that invocation.
An empty override or a GitHub App client ID fails before requesting authorization.

No token, password, device secret, or refresh token is stored in the catalog or
workspace registry. Managed connector credentials are keyed by connector UUID
under the native service `io.github.programmablemike.spacer.github`:

| Platform | Managed credential storage |
| --- | --- |
| macOS | Keychain |
| Windows | Credential Manager |
| Linux | Secret Service over D-Bus, with encrypted secret transport |

Missing, locked, or inaccessible storage produces an authentication error. Spacer
has no plaintext or mock-store fallback in production. Linux requires a running
Secret Service provider in the user's session. A headless machine can use an
explicit GitHub CLI connector if its existing credential setup is available.

Access and refresh tokens stay in the native adapter. Device authorizations are
held in memory until completion or cancellation; restarting Spacer requires a new
sign-in. OAuth polling obeys GitHub's interval, expiry, and slow-down responses.
Managed credentials refresh under a per-connector process lock because refresh
tokens rotate. Every remote operation verifies the selected credential's stable
account ID; it never falls back to another account.

## Workspace bindings

Workspace schema 2 adds optional `bindings` and `setup_defaults` arrays. Local-only
workspaces continue writing schema 1 until these features are used. Spacer reads
legacy schemas 0/1 and schema 2; older binaries cannot read schema 2 and should not
be used to edit it. Back up the registry before upgrading or restoring a previous
binary.

A project binding records the space/project pair, connector and identity IDs,
optional remote repository metadata, and the last applied commit name/email. The
repository ID remains distinct from its mutable owner/name and clone URL.
Selections are recorded when provisioning rather than resolving new defaults for
existing projects. Missing catalog references do not block local operations.

Setup defaults are independent connector and identity references, scoped to the
workspace (`space: null`) or a named space. An explicit setup selection overrides
the space default, which overrides the workspace default. Explicitly keeping Git
configuration overrides identity defaults for that operation. Unset default fields
inherit from the workspace. Defaults never change repository authorship by themselves.

## Commands

| Command | Effect |
| --- | --- |
| `identity list` | Show saved identities, names, and emails |
| `identity add LABEL --name NAME --email EMAIL` | Save an identity without changing Git |
| `identity remove ID_OR_LABEL` | Remove metadata; preserve Git configuration |
| `identity apply ID_OR_LABEL --space SPACE --project PROJECT [--yes]` | Preview values; with `--yes`, apply local authorship and record the binding |
| `connector list` | Show account metadata without credentials |
| `connector existing` | Show supported GitHub CLI accounts |
| `connector link LABEL --account LOGIN` | Validate and link a specific existing account |
| `connector login LABEL [--client-id ID] [--no-browser]` | Complete device authorization and store managed credentials |
| `connector disconnect ID_OR_LABEL` | Remove connection and owned credentials |
| `connector repositories ID_OR_LABEL` | List accessible stable IDs, names, and visibility |
| `setup [--root PATH] [--space SPACE] [--connector ID_OR_LABEL] [--identity ID_OR_LABEL \| --keep-git-identity] [--repository ID]... [--project-name ID=NAME]... [--apply] [--save-defaults]` | Review or provision GitHub projects; prompt for missing choices in a terminal |

The native credential helper is an internal protocol entry point shared by the
CLI and desktop executable. It returns credentials only for a Git request matching
HTTPS, `github.com`, and the exact planned repository path. It is scoped to one
clone command; credentials are not installed into a repository or global helper
configuration. The desktop frontend can access only metadata and opaque plan IDs.
