use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use std::io::Write as _;
use std::process::Command;

#[derive(Args)]
pub struct ShellArgs {
    /// Shell binary to spawn (defaults to $SHELL, then sh)
    #[arg(long)]
    pub shell: Option<String>,
    #[command(subcommand)]
    pub command: Option<ShellCommands>,
}

#[derive(Subcommand)]
pub enum ShellCommands {
    /// Print shell integration code to eval in your rc file
    Integration {
        /// Shell to generate integration for: zsh, bash, or sh.
        /// Detected from $SHELL if omitted.
        shell: Option<String>,
    },
}

pub fn run(args: ShellArgs) -> Result<()> {
    match args.command {
        Some(ShellCommands::Integration { shell }) => integration(shell),
        None => enter(args.shell),
    }
}

// ---------------------------------------------------------------------------
// Managed shell entry
// ---------------------------------------------------------------------------

fn enter(shell_flag: Option<String>) -> Result<()> {
    if std::env::var("SPACER_SHELL_ACTIVE").is_ok() {
        bail!("already inside a managed spacer shell — run `exit` to leave it first");
    }

    // Detect if we're being run inside $(...) — e.g. eval "$(spacer shell)".
    // In that case stdout is a pipe, not a terminal. Emit a helpful error
    // rather than trying to spawn an interactive shell into a pipe.
    if cfg!(windows) {
        bail!("managed shells are supported on macOS/Linux; use spacer path on Windows");
    }
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        bail!("managed shells require a terminal; use `spacer shell integration` to print integration code");
    }
    let shell_path = shell_flag
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "sh".to_string());

    let shell_name = std::path::Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh")
        .to_string();

    let root = crate::composition::open_workspace(false)?.root().to_owned();

    #[cfg(unix)]
    let terminal_settings = TerminalSettings::capture()?;
    let exit_code = match shell_name.as_str() {
        "bash" => enter_bash(&shell_path, &root)?,
        "zsh" => enter_zsh(&shell_path, &root)?,
        _ => enter_sh(&shell_path, &root)?,
    };

    #[cfg(unix)]
    drop(terminal_settings);
    std::process::exit(exit_code);
}

fn enter_bash(shell_path: &str, root: &std::path::Path) -> Result<i32> {
    let mut init_file = tempfile::Builder::new()
        .prefix("spacer-bash-")
        .suffix(".sh")
        .tempfile()?;

    let content = format!(
        "{source_rc}\n{bootstrap}\n",
        source_rc = source_if_exists(home_file(".bashrc").as_deref()),
        bootstrap = bootstrap_content("bash"),
    );
    init_file.write_all(content.as_bytes())?;
    init_file.flush()?;

    // init_file stays alive until the shell exits (tempfile deleted on drop)
    let status = Command::new(shell_path)
        .arg("--init-file")
        .arg(init_file.path())
        .arg("-i")
        .env("SPACER_SHELL_ACTIVE", "1")
        .env("SPACER_ROOT", root)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

fn enter_zsh(shell_path: &str, root: &std::path::Path) -> Result<i32> {
    let zdotdir = tempfile::tempdir()?;

    std::fs::write(
        zdotdir.path().join(".zshenv"),
        format!(
            "{}\nexport ZDOTDIR={}\n",
            source_if_exists(zsh_file(".zshenv").as_deref()),
            quote(&zdotdir.path().to_string_lossy())
        ),
    )?;

    std::fs::write(
        zdotdir.path().join(".zshrc"),
        format!(
            "{source_rc}\n{bootstrap}\n",
            source_rc = format_args!(
                "{}\n{}",
                original_zdotdir(),
                source_if_exists(zsh_file(".zshrc").as_deref())
            ),
            bootstrap = bootstrap_content("zsh"),
        ),
    )?;

    // Skip global interactive startup; explicitly preserve user startup files.
    // zdotdir stays alive until the shell exits
    let status = Command::new(shell_path)
        .arg("-d")
        .arg("-i")
        .env("SPACER_SHELL_ACTIVE", "1")
        .env("ZDOTDIR", zdotdir.path())
        .env("SPACER_ROOT", root)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

fn enter_sh(shell_path: &str, root: &std::path::Path) -> Result<i32> {
    // POSIX sh has no reliable bootstrap injection; set env vars only.
    let status = Command::new(shell_path)
        .env("SPACER_SHELL_ACTIVE", "1")
        .env("SPACER_ROOT", root)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

// ---------------------------------------------------------------------------
// Bootstrap content injected into managed bash/zsh sessions
// ---------------------------------------------------------------------------

fn bootstrap_content(shell_name: &str) -> String {
    let prompt_indicator = match shell_name {
        // Register a precmd hook that runs after Starship (or any other
        // framework) so RPROMPT is set last and not overwritten.
        "zsh" => {
            "__spacer_precmd() { RPROMPT=\"%F{cyan}(spacer)%f\" }\n\
             precmd_functions+=(__spacer_precmd)\n"
        }
        // Append to PROMPT_COMMAND so we run after Starship updates PS1,
        // then print a dim indicator line above the prompt.
        "bash" => {
            "__spacer_prompt() { printf '\\e[2m(spacer)\\e[0m\\n'; }\n\
             PROMPT_COMMAND=\"${PROMPT_COMMAND:+$PROMPT_COMMAND; }__spacer_prompt\"\n"
        }
        _ => "",
    };

    // Always inject the cd-intercepting wrapper so `spacer space go` /
    // `spacer project go` change directories regardless of whether the user
    // has `eval "$(spacer shell integration)"` in their rc file.
    let nav_wrapper = match shell_name {
        "zsh" | "bash" => INTEGRATION_BASH_ZSH,
        _ => INTEGRATION_SH,
    };

    format!(
        "export SPACER_SHELL_ACTIVE=1\n{nav_wrapper}\n{prompt_indicator}{completions}",
        nav_wrapper = nav_wrapper,
        prompt_indicator = prompt_indicator,
        completions = completions_hook(shell_name),
    )
}

fn completions_hook(shell_name: &str) -> &'static str {
    match shell_name {
        "bash" => {
            "if command -v spacer >/dev/null 2>&1; then\n\
             \t_spacer_completion=$(command spacer completions bash 2>/dev/null) && eval \"$_spacer_completion\"; unset _spacer_completion\n\
             fi"
        }
        "zsh" => {
            "if command -v spacer >/dev/null 2>&1; then\n\
             \t(( $+functions[compdef] )) || { autoload -Uz compinit; compinit -i; }\n\
             \t_spacer_completion=$(command spacer completions zsh 2>/dev/null) && eval \"$_spacer_completion\"; unset _spacer_completion\n\
             fi"
        }
        _ => "",
    }
}

fn home_file(filename: &str) -> Option<String> {
    std::env::var("HOME")
        .ok()
        .map(|home| format!("{}/{}", home, filename))
}

fn source_if_exists(path: Option<&str>) -> String {
    match path {
        Some(p) => format!("[ ! -f {p} ] || . {p}", p = quote(p)),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Shell integration snippet (eval-able, for rc files)
// ---------------------------------------------------------------------------

fn integration(shell_flag: Option<String>) -> Result<()> {
    let shell = shell_flag
        .or_else(|| {
            std::env::var("SHELL")
                .ok()
                .and_then(|s| s.rsplit('/').next().map(str::to_string))
        })
        .unwrap_or_else(|| "sh".to_string());

    let snippet = match shell.as_str() {
        "zsh" | "bash" => INTEGRATION_BASH_ZSH,
        "sh" => INTEGRATION_SH,
        other => bail!("unsupported shell '{}' — supported: zsh, bash, sh", other),
    };

    println!("{}", snippet);
    Ok(())
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn zsh_file(filename: &str) -> Option<String> {
    std::env::var("ZDOTDIR")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .map(|dir| format!("{dir}/{filename}"))
}
fn original_zdotdir() -> String {
    match std::env::var("ZDOTDIR") {
        Ok(value) => format!("export ZDOTDIR={}", quote(&value)),
        Err(_) => "unset ZDOTDIR".into(),
    }
}

const INTEGRATION_BASH_ZSH: &str = INTEGRATION_SH;
const INTEGRATION_SH: &str = r#"spacer() {
    case "${1-}:${2-}" in
        go:*|space:go|project:go)
            # Assignment preserves the resolver status; cd never runs on failure.
            _spacer_target=$(command spacer "$@") || return $?
            [ -n "$_spacer_target" ] || return 1
            cd -- "$_spacer_target"
            ;;
        *) command spacer "$@" ;;
    esac
}"#;

#[cfg(unix)]
struct TerminalSettings(libc::termios);
#[cfg(unix)]
impl TerminalSettings {
    fn capture() -> Result<Self> {
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes the provided termios on success.
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, settings.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: the successful call above initialized the value.
        Ok(Self(unsafe { settings.assume_init() }))
    }
}
#[cfg(unix)]
impl Drop for TerminalSettings {
    fn drop(&mut self) {
        // SAFETY: self.0 is a valid termios captured from the still-open stdin.
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_paths_are_shell_quoted() {
        assert_eq!(quote("a'b$(x)"), "'a'\"'\"'b$(x)'");
        assert!(source_if_exists(Some("a b")).contains("'a b'"));
    }
}
