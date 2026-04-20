use anyhow::{bail, Result};
use clap::Args;

#[derive(Args)]
pub struct ShellArgs {
    /// Shell to generate integration for: zsh, bash, or sh.
    /// Detected from $SHELL if omitted.
    pub shell: Option<String>,
}

pub fn run(args: ShellArgs) -> Result<()> {
    let shell = args.shell
        .or_else(|| {
            std::env::var("SHELL").ok().and_then(|s| {
                s.rsplit('/').next().map(|s| s.to_string())
            })
        })
        .unwrap_or_else(|| "sh".to_string());

    let snippet = match shell.as_str() {
        "zsh" | "bash" => BASH_ZSH,
        "sh" => SH,
        other => bail!("unsupported shell '{}' — supported: zsh, bash, sh", other),
    };

    println!("{}", snippet);
    Ok(())
}

// zsh and bash share the same syntax for this wrapper
const BASH_ZSH: &str = r#"# Spacer shell integration — add to your .zshrc / .bashrc:
#   eval "$(spacer shell)"
function spacer() {
    if [[ "$1" == "space" && "$2" == "go" ]]; then
        cd "$(command spacer space go "${@:3}")"
    elif [[ "$1" == "project" && "$2" == "go" ]]; then
        cd "$(command spacer project go "${@:3}")"
    else
        command spacer "$@"
    fi
}"#;

const SH: &str = r#"# Spacer shell integration — add to your .profile:
#   eval "$(spacer shell sh)"
spacer() {
    if [ "$1" = "space" ] && [ "$2" = "go" ]; then
        shift 2
        cd "$(command spacer space go "$@")"
    elif [ "$1" = "project" ] && [ "$2" = "go" ]; then
        shift 2
        cd "$(command spacer project go "$@")"
    else
        command spacer "$@"
    fi
}"#;
