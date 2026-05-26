use std::ffi::OsStr;
use std::io::{self, Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
mod helper;
#[cfg(target_os = "windows")]
mod winapi;

const GIT_DIR_NAME: &str = ".cfg";
const NO_SHOW_UNTRACKED: &[&str] = &["config", "status.showUntrackedFiles", "no"];

fn main() -> Result<()> {
    let action = cli_parse();
    let Some(action) = action else {
        show_help(1);
    };

    cli_run(action, CliEnv::new())
}

fn cli_parse() -> Option<CliAction> {
    let mut args = std::env::args().skip(1).peekable();
    let arg = args.peek()?;
    let action = match arg.as_str() {
        "--init" => CliAction::Init,

        "--clone" => {
            args.next();
            let url = args.next()?;
            CliAction::Clone(url, args.next())
        }

        #[cfg(target_os = "windows")]
        "-H" | "--hide" => {
            args.next();
            let path = args.next()?;
            CliAction::HideDotfile(path, args.next().is_some_and(|s| &s == "no"))
        }

        "-h" | "--help" => {
            show_help(0);
        }

        "lz" | "lazy" | "lazygit" => {
            args.next();
            let args: Option<Vec<String>> = args.peek().is_some().then_some(args.collect());
            CliAction::RunLazyGit(args)
        }

        _ => {
            let args: Vec<String> = args.collect();
            CliAction::RunGit(args)
        }
    };
    Some(action)
}

fn cli_run(action: CliAction, env: CliEnv) -> Result<()> {
    let git_dir = path_try_to_str(&env.git_dir)?;
    let mut git = GitWrapper::new();

    match action {
        CliAction::Init => {
            let gitignore = env.work_tree.join(".gitignore");

            let steps: [&[&str]; 4] = [
                &["init", "--bare", git_dir],
                &["add", path_try_to_str(&gitignore)?],
                &["commit", "-m", "ignore git dir"],
                NO_SHOW_UNTRACKED,
            ];

            git.run(steps[0])?;

            let content = format!("{}\n", GIT_DIR_NAME);

            if gitignore.exists() {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&gitignore)?
                    .write_all(content.as_bytes())?;
            } else {
                std::fs::write(&gitignore, content)?;
            }

            for args in &steps[1..] {
                git.run_with_env(&env, *args)?;
            }
        }

        CliAction::Clone(url, branch) => {
            let steps: [&[&str]; 4] = [
                &["clone", "--bare", url.as_str(), git_dir],
                &[
                    "config",
                    "remote.origin.fetch",
                    "+refs/heads/*:refs/remotes/origin/*",
                ],
                &["fetch"],
                NO_SHOW_UNTRACKED,
            ];

            for args in steps {
                git.run_with_env(&env, args)?;
            }

            if let Some(branch) = branch {
                git.run_with_env(&env, ["checkout", branch.as_str()])?;
            }
        }

        #[cfg(target_os = "windows")]
        CliAction::HideDotfile(path, was_hidden) => {
            use helper::*;
            set_dotfile_attr_in_dir(
                path.into(),
                if !was_hidden {
                    set_hidden
                } else {
                    unset_hidden
                },
            )?;
        }

        CliAction::RunGit(args) => {
            git.run_with_env(&env, args)?;
        }

        CliAction::RunLazyGit(args) => {
            git.cmd = Command::new("lazygit");
            git.with_env(&env);
            if let Some(args) = args {
                git.cmd.args(args);
            }
            git.spawn()?;
        }
    }
    Ok(())
}

impl GitWrapper {
    fn new() -> Self {
        let cmd = Command::new("git");
        Self { cmd }
    }

    fn with_env(&mut self, env: &CliEnv) {
        self.cmd
            .env("GIT_DIR", &env.git_dir)
            .env("GIT_WORK_TREE", &env.work_tree);
    }

    fn run_with_env<I, S>(&mut self, env: &CliEnv, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.cmd = Command::new("git");
        self.with_env(env);
        self.run(args)
    }

    fn run<I, S>(&mut self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.cmd.args(args);
        self.spawn()
    }

    fn spawn(&mut self) -> Result<()> {
        self.cmd.spawn()?.wait().map(|_| ())
    }
}

struct GitWrapper {
    cmd: Command,
}

struct CliEnv {
    git_dir: PathBuf,
    work_tree: PathBuf,
}

impl CliEnv {
    fn new() -> Self {
        let debug = std::env::var("DFM_DEBUG").is_ok();
        let dir = if !debug {
            std::env::home_dir()
        } else {
            std::env::current_dir().ok()
        };

        let work_tree = dir.expect("Impossible to get your home dir!");
        let git_dir = work_tree.join(GIT_DIR_NAME);
        Self { git_dir, work_tree }
    }
}

enum CliAction {
    Init,
    #[cfg(target_os = "windows")]
    HideDotfile(String, bool),
    Clone(String, Option<String>),
    RunGit(Vec<String>),
    RunLazyGit(Option<Vec<String>>),
}

pub fn path_try_to_str(s: &Path) -> Result<&str> {
    <&str>::try_from(s.as_os_str()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
}

fn show_help(code: i32) -> ! {
    let msg = "Usage: dfm <Flag> or <Commands>

Flags:
  --init                  Initialize a new dotfile repository in $HOME
  --clone <url> [branch]  Clone an existing dotfile repository,
                          after checkout branch(optional)
  -H|--hide <dir> [no]    Hidden dotfiles in <dir>(Windows only).
                          if with [no], unset hidden.
  -h|--help               Show this help message

Commands:
  lz|lazy|lazygit     Launch lazygit with dotfile environment
  <git args>          Pass through to git with environment set

Environment Variables:
  DFM_DEBUG=[any]  if set, using current dir. just for me debugging:)";

    eprintln!("{msg}");
    std::process::exit(code);
}
