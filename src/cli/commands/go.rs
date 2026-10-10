use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct GoArgs {
    /// Target to navigate to: <space>[/<project>[/<change>]]
    ///
    /// Examples:
    ///   spacer go my-org
    ///   spacer go my-org/my-repo
    ///   spacer go my-org/my-repo/feat/login
    pub target: String,
}

pub fn run(args: GoArgs, ws: &spacer_core::Workspace) -> Result<()> {
    let mut parts = args.target.splitn(3, '/');
    let space = parts.next().unwrap_or("");
    let path = match (parts.next(), parts.next()) {
        (Some(project), Some(change)) => ws.change_path(change, project, space)?,
        (Some(project), None) => ws.project_path(project, space)?,
        (None, _) => ws.space_path(space)?,
    };
    print!("{}", path.display());
    Ok(())
}
