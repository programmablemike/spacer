use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use spacer_core::connections::{Connections, SetupDefault};
use std::io::{IsTerminal, Write};

#[derive(Args)]
pub struct IdentityArgs {
    #[command(subcommand)]
    pub command: IdentityCommand,
}
#[derive(Subcommand)]
pub enum IdentityCommand {
    List,
    Add {
        label: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
    },
    Remove {
        identity: String,
    },
    /// Review and explicitly apply repository-local commit authorship
    Apply {
        identity: String,
        #[arg(long)]
        space: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        yes: bool,
    },
}
#[derive(Args)]
pub struct ConnectorArgs {
    #[command(subcommand)]
    pub command: ConnectorCommand,
}
#[derive(Subcommand)]
pub enum ConnectorCommand {
    List,
    /// List accounts exposed by GitHub CLI
    Existing,
    /// Use one GitHub CLI account without switching its active account
    Link {
        label: String,
        #[arg(long)]
        account: String,
    },
    /// Authorize Spacer through GitHub's device flow
    Login {
        label: String,
        #[arg(long)]
        client_id: Option<String>,
        #[arg(long)]
        no_browser: bool,
    },
    Disconnect {
        connector: String,
    },
    Repositories {
        connector: String,
    },
}
pub fn identities(args: IdentityArgs) -> Result<()> {
    let service = crate::composition::connections()?;
    match args.command {
        IdentityCommand::List => {
            for i in service.snapshot()?.identities {
                println!("{}\t{}\t{} <{}>", i.id, i.label, i.name, i.email);
            }
        }
        IdentityCommand::Add { label, name, email } => {
            let i = service.add_identity(&label, &name, &email)?;
            println!("Identity {}: {} <{}>", i.id, i.name, i.email);
        }
        IdentityCommand::Remove { identity } => {
            let i = service.identity(&identity)?;
            service.remove_identity(&i.id)?;
            println!("Identity removed; repository Git configuration is preserved.");
        }
        IdentityCommand::Apply {
            identity,
            space,
            project,
            yes,
        } => {
            let mut workspace = crate::composition::open_workspace(false)?;
            let identity = service.identity(&identity)?;
            let current = workspace.project_identity(&space, &project)?;
            println!(
                "Current: {} <{}>\nProposed: {} <{}>",
                current.0.as_deref().unwrap_or("unset"),
                current.1.as_deref().unwrap_or("unset"),
                identity.name,
                identity.email
            );
            anyhow::ensure!(
                yes,
                "review these values and repeat with --yes to apply repository-local authorship"
            );
            service.apply_identity(&mut workspace, &space, &project, &identity.id)?;
            println!("Repository-local commit identity applied.");
        }
    }
    Ok(())
}
pub fn connectors(args: ConnectorArgs) -> Result<()> {
    let mut service = crate::composition::connections()?;
    match args.command {
        ConnectorCommand::List => {
            for c in service.snapshot()?.connectors {
                println!("{}\t{}\t{}\t{}", c.id, c.label, c.host, c.login);
            }
        }
        ConnectorCommand::Existing => {
            for a in service.existing_accounts()? {
                println!("{}", a.login);
            }
        }
        ConnectorCommand::Link { label, account } => {
            let c = service.link_existing(&label, &account)?;
            println!("Connected {} as {} ({})", c.label, c.login, c.id);
        }
        ConnectorCommand::Login {
            label,
            client_id,
            no_browser,
        } => login(&mut service, &label, client_id.as_deref(), no_browser)?,
        ConnectorCommand::Disconnect { connector } => {
            let c = service.connector(&connector)?;
            service.disconnect(&c.id)?;
            println!("Disconnected; existing project files and Git configuration are preserved.");
        }
        ConnectorCommand::Repositories { connector } => {
            for r in service.repositories(&connector)? {
                println!(
                    "{}\t{}\t{}",
                    r.id,
                    r.full_name,
                    if r.private { "private" } else { "public" }
                );
            }
        }
    }
    Ok(())
}
fn login(
    service: &mut Connections,
    label: &str,
    client_id: Option<&str>,
    no_browser: bool,
) -> Result<()> {
    let client_id = spacer_data::github_client_id(client_id)?;
    println!("GitHub OAuth authorization requests read and write repository access (repo), including private repositories, within your existing permissions. Organizations may require OAuth approval.");
    let challenge = service.begin_login(label, &client_id)?;
    println!(
        "Open {} and enter {}. Authorize the account you intend to connect.",
        challenge.verification_uri, challenge.user_code
    );
    if !no_browser {
        let _ = open::that(&challenge.verification_uri);
    }
    loop {
        std::thread::sleep(std::time::Duration::from_secs(challenge.interval));
        match service.poll_login(&challenge.id) {
            Ok(Some(c)) => {
                println!("Connected {} as {} ({})", c.label, c.login, c.id);
                return Ok(());
            }
            Ok(None) => (),
            Err(e) => {
                let _ = service.cancel_login(&challenge.id);
                return Err(e.into());
            }
        }
    }
}
#[derive(Args)]
pub struct SetupArgs {
    #[arg(long)]
    pub root: Option<std::path::PathBuf>,
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long)]
    pub connector: Option<String>,
    #[arg(long)]
    pub identity: Option<String>,
    /// Retain ordinary Git authorship configuration instead of a managed identity
    #[arg(long, conflicts_with = "identity")]
    pub keep_git_identity: bool,
    /// Stable repository IDs; repeat for multiple projects
    #[arg(long)]
    pub repository: Vec<String>,
    /// Override a local project name: --project-name REPOSITORY_ID=NAME
    #[arg(long)]
    pub project_name: Vec<String>,
    /// Apply the reviewed plan (otherwise only preview)
    #[arg(long)]
    pub apply: bool,
    /// Save connector and identity defaults for future setup in this space
    #[arg(long)]
    pub save_defaults: bool,
}
fn prompt(question: &str) -> Result<String> {
    anyhow::ensure!(
        std::io::stdin().is_terminal(),
        "missing setup argument: {question}; supply explicit flags when stdin is not a terminal"
    );
    print!("{question}: ");
    std::io::stdout().flush()?;
    let mut value = String::new();
    std::io::stdin().read_line(&mut value)?;
    Ok(value.trim().into())
}
pub fn setup(mut args: SetupArgs) -> Result<()> {
    let mut accounts = crate::composition::connections()?;
    let root = match args.root {
        Some(root) => root,
        None => prompt("Workspace folder")?.into(),
    };
    let root = spacer_core::ports::Filesystem::normalize(&spacer_data::LocalFilesystem, &root)?;
    anyhow::ensure!(root.is_dir(), "choose an existing workspace folder");
    let space = args
        .space
        .take()
        .map(Ok)
        .unwrap_or_else(|| prompt("Space name"))?;
    let mut workspace = crate::composition::open_root(root.clone())?;
    if args.connector.is_none() {
        for c in accounts.snapshot()?.connectors {
            println!("{}: {} ({})", c.id, c.label, c.login);
        }
        let choice = prompt("Connector label/ID, or 'login' to connect GitHub")?;
        if choice == "login" {
            let label = prompt("Connection label")?;
            login(&mut accounts, &label, None, false)?;
            args.connector = Some(label);
        } else {
            args.connector = Some(choice);
        }
    }
    let connector = accounts.connector(args.connector.as_deref().context("select a connector")?)?;
    if args.identity.is_none() && !args.keep_git_identity {
        for i in accounts.snapshot()?.identities {
            println!("{}: {} = {} <{}>", i.id, i.label, i.name, i.email);
        }
        let choice = prompt("Identity label/ID, 'new', or 'keep' for ordinary Git configuration")?;
        if choice == "new" {
            let label = prompt("Identity label")?;
            let name = prompt("Commit name")?;
            let email = prompt("Commit email")?;
            args.identity = Some(accounts.add_identity(&label, &name, &email)?.id);
        } else if choice != "keep" {
            args.identity = Some(choice);
        } else {
            args.keep_git_identity = true;
        }
    }
    let identity = args
        .identity
        .as_deref()
        .map(|id| accounts.identity(id))
        .transpose()?;
    let repositories = accounts.repositories(&connector.id)?;
    if args.repository.is_empty() {
        for r in &repositories {
            println!("{}: {}", r.id, r.full_name);
        }
        args.repository = prompt("Repository IDs separated by commas")?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }
    let selected = args
        .repository
        .iter()
        .map(|id| {
            repositories
                .iter()
                .find(|r| &r.id == id)
                .cloned()
                .context("repository is not available through this connector")
        })
        .collect::<Result<Vec<_>>>()?;
    let mut overrides = std::collections::HashMap::new();
    for entry in &args.project_name {
        let (id, name) = entry
            .split_once('=')
            .context("project-name must be REPOSITORY_ID=NAME")?;
        anyhow::ensure!(
            overrides.insert(id, name).is_none(),
            "duplicate project-name override"
        );
        anyhow::ensure!(
            selected.iter().any(|r| r.id == id),
            "project-name refers to an unselected repository"
        );
    }
    let mut names = std::collections::HashSet::new();
    for r in &selected {
        let name = overrides.get(r.id.as_str()).copied().unwrap_or(
            r.full_name
                .split('/')
                .next_back()
                .context("invalid repository name")?,
        );
        anyhow::ensure!(
            names.insert(name),
            "selected repositories share a local name; import them into separate spaces"
        );
        println!(
            "{} -> {} | connector: {} ({}) | identity: {}",
            r.full_name,
            workspace
                .preview_import_destination(name, &space)?
                .display(),
            connector.label,
            connector.login,
            identity
                .as_ref()
                .map(|i| format!("{} <{}>", i.name, i.email))
                .unwrap_or_else(|| "existing Git configuration".into())
        );
    }
    if !args.apply {
        println!("Review this plan and repeat with --apply to set up the projects.");
        return Ok(());
    }
    workspace.initialize()?;
    if !workspace.spaces().iter().any(|s| s.name == space) {
        workspace.create_space(&space, None)?;
    }
    // Build every plan before cloning any project; collisions fail before effects.
    let plans = selected
        .iter()
        .map(|r| {
            accounts.plan_import(
                &workspace,
                &space,
                overrides
                    .get(r.id.as_str())
                    .copied()
                    .unwrap_or(r.full_name.split('/').next_back().unwrap()),
                Some(&connector.id),
                Some(identity.as_ref().map(|i| i.id.as_str()).unwrap_or("")),
                &r.id,
            )
        })
        .collect::<spacer_core::Result<Vec<_>>>()?;
    let mut failures = 0;
    for plan in plans {
        match accounts.import(&mut workspace, &plan) {
            Ok(()) => println!("Ready: {}/{}", space, plan.project),
            Err(e) => {
                failures += 1;
                eprintln!("{}: {e}", plan.project);
            }
        }
    }
    if args.save_defaults {
        workspace.set_setup_defaults(SetupDefault {
            space: Some(space),
            connector: Some(connector.id),
            identity: identity.map(|i| i.id),
        })?;
    }
    anyhow::ensure!(
        failures == 0,
        "{failures} projects need recovery; completed projects remain available"
    );
    Ok(())
}
