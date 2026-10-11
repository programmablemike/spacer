# Set up projects from GitHub

[Documentation](../README.md) / How-to

A connector selects the GitHub account used to discover and clone repositories.
A commit identity selects the name and email Git records on commits. Choose these
independently; connecting GitHub does not configure commit authorship.

## Connect an account

Spacer supports GitHub.com. Its registered GitHub App must have **Device flow**
enabled and be installed on the accounts/repositories you want to access. Grant
repository metadata and Contents read access for discovery and cloning. An
organization may require administrator approval or SSO before repositories appear.
See [GitHub App device authorization](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token).

In the desktop, open or create a workspace, then choose **Set up from GitHub**.
Choose **Sign in with GitHub**, open the system browser, and enter the displayed
code. Verify the account in the browser before authorizing. After authorization,
Spacer shows the verified account and saves its tokens in the system credential
manager. You can cancel while waiting for authorization.

From the CLI, connection management works outside an initialized workspace:

```sh
spacer connector login personal
spacer connector list
spacer connector repositories personal
```

Use `--no-browser` when you want to open the displayed verification URL yourself.
The public GitHub App client ID is bundled with Spacer. To use another registered
App, set `SPACER_GITHUB_CLIENT_ID` or pass `--client-id` to `connector login`.
The numeric App ID is not the client ID. An App client secret is not required
for this device flow and must not be supplied.

Alternatively, choose **Find existing GitHub CLI accounts** and select one, or:

```sh
spacer connector existing
spacer connector link work --account example-account
```

This keeps the credentials managed by GitHub CLI. Spacer requests that specific
account, verifies it, and leaves GitHub CLI's active account unchanged. GitHub CLI
must be on the native process's PATH, including when launching the desktop from a
menu. Existing Spacer connectors are always selectable through the user catalog.
GitHub CLI's own credential storage policy applies to linked connections; Spacer
neither copies those credentials nor introduces a plaintext fallback.

## Select commit authorship

In setup, select an existing identity, create one, or choose **Keep existing Git
configuration**. A new identity requires an explicit label, commit name, and
email. Review the exact values before applying them.

```sh
spacer identity add personal --name example-account --email example-account@users.noreply.github.com
spacer identity list
```

For an existing project, choose **Commit identity** in its project view. The
current values are shown before applying a saved identity. The CLI also requires
explicit confirmation:

```sh
spacer identity apply personal --space personal --project example
# Review the proposed values, then:
spacer identity apply personal --space personal --project example --yes
```

Spacer atomically updates the repository's local `user.name` and `user.email`.
Linked worktrees share this configuration. Global Git settings and signing
configuration are preserved. Worktree-specific authorship settings must be
reconciled with Git before applying a project identity. If authorship was changed
outside Spacer after an identity was applied, Spacer reports the conflict instead
of overwriting it.

## Review and provision projects

Select repositories, choose a space (or explicitly create one), choose authorship,
and review the repository, destination, connector account, and exact commit name
and email. **Set up projects** clones and registers them one at a time. **Stop
before next project** lets the current clone finish and skips subsequent projects.
Completed projects remain available even if another fails.

The CLI can guide setup interactively:

```sh
spacer setup
```

It prompts for missing selections and prints a plan. Repeat with `--apply` and the
same explicit choices to execute it. For scripting, first obtain stable repository
IDs from `connector repositories`, then preview:

```sh
spacer setup --root /absolute/path/to/workspace --space personal \
  --connector personal --identity personal --repository 123456789
```

Add `--apply` to initialize the workspace if needed, create the space if needed,
and provision the selected projects. Repeat `--repository` for multiple projects.
Use `--keep-git-identity` to explicitly retain ordinary Git configuration. An
existing custom space path is respected. Selected repositories with identical
local names need distinct names or separate spaces. Use
`--project-name REPOSITORY_ID=LOCAL_NAME` to override a name; the desktop offers
a local project name field for each selection.

`--save-defaults` saves the connector and identity selection for future service
operations in that space. Changing defaults does not reconfigure existing projects.

## Recovery and disconnection

If a clone or registration fails, inspect the reported destination. Spacer never
deletes project files as rollback. A complete checkout whose registration failed
can be registered using the existing project workflow:

```sh
spacer project register example --space personal --path /absolute/path/to/checkout
spacer identity apply personal --space personal --project example --yes
```

Manual registration restores local management; it does not reconstruct a failed
import's remote connector binding. Retrying setup refuses a nonempty destination
and does not reclone over existing files. See [workspace recovery](recover-workspace.md).

```sh
spacer connector disconnect personal
spacer identity remove personal
```

Disconnecting deletes Spacer-owned keychain entries, but leaves GitHub CLI
credentials untouched. It does not revoke the GitHub App's server-side grant;
manage that in GitHub settings. Removing a connection or identity preserves project
files and existing Git authorship. Local operations remain usable offline or with
missing credentials. Reconnect explicitly before further remote operations.

This workflow imports existing repositories over HTTPS. Remote repository creation,
GitHub Enterprise, SSH account selection, commit signing profiles, and using Spacer
credentials for ordinary shell `git push` commands are not implemented.
