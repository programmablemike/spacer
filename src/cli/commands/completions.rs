use anyhow::{bail, Result};
use clap::Args;
use clap_complete::generate;
use clap_complete::shells::{Bash, Fish, Zsh};
use std::io::stdout;

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for: bash, zsh, or fish
    pub shell: String,
}

/// Generate shell completions. Accepts the root `clap::Command` from the
/// caller so this module does not create a circular dependency on `crate::cli`.
pub fn run(args: CompletionsArgs, cmd: &mut clap::Command) -> Result<()> {
    match args.shell.as_str() {
        "bash" => generate(Bash, cmd, "spacer", &mut stdout()),
        "zsh" => generate(Zsh, cmd, "spacer", &mut stdout()),
        "fish" => generate(Fish, cmd, "spacer", &mut stdout()),
        other => bail!("unsupported shell '{}' — supported: bash, zsh, fish", other),
    }
    Ok(())
}
