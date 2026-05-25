use std::io::{Result, Write};
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
mod winapi;
#[cfg(target_os = "windows")]
mod helper;

const DIR_NAME: &str = ".cfg";
const NO_SHOW_UNTRACKED: &[&str] = &["config", "status.showUntrackedFiles", "no"];

fn main() -> Result<()> {
    let action = cli_parse();
    let Some(action) = action else {
        help(1);
    };

    run(action, CliEnv::new())
}

fn cli_parse() -> Option<CliAction> {
    let mut args = std::env::args().skip(1).peekable();
    let arg = args.peek()?;
    let action = match arg.as_str() {
        "--init" => CliAction::Init,

        "--clone" => {
            args.next();
            let url = args.next()?;
            CliAction::Clone(url)
        }

        #[cfg(target_os = "windows")]
        "--hide-dotfile" => {
            args.next();
            let path = args.next()?;
            CliAction::HideDotfile(path)
        },

        "-h" | "--help" => {
            help(0);
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

fn run(action: CliAction, env: CliEnv) -> Result<()> {
    let git_dir = env.git_dir.to_str().unwrap();
    let mut git = GitWrapper::new();

    match action {
        CliAction::Init => {
            git.run(&["init", "--bare", git_dir])?;

            let gitignore = env.work_tree.join(".gitignore");
            let content = format!("{}\n", DIR_NAME);

            if gitignore.exists() {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&gitignore)?
                    .write_all(content.as_bytes())?;
            } else {
                std::fs::write(&gitignore, content)?;
            }

            let steps: [&[&str]; 3] = [
                &["add", gitignore.to_str().unwrap()],
                &["commit", "-m", "ignore git dir"],
                NO_SHOW_UNTRACKED,
            ];
            for args in steps {
                git.run_with_env(&env, args)?;
            }
        }

        #[cfg(target_os = "windows")]
        CliAction::HideDotfile(path) => {
            helper::hide_dotfile_in_dir(path)?;
        }

        CliAction::Clone(url) => {
            git.run(&["clone", "--bare", url.as_str(), git_dir])?;

            let steps: [&[&str]; 5] = [
                &["checkout"],
                &[
                    "config",
                    "remote.origin.fetch",
                    "+refs/heads/*:refs/remotes/origin/*",
                ],
                &["fetch", "origin"],
                &["branch", "-u", "origin/main"],
                NO_SHOW_UNTRACKED,
            ];
            for args in steps {
                git.run_with_env(&env, args)?;
            }
        }
        CliAction::RunGit(args) => {
            git.with_env(&env);
            git.cmd.args(args);
            git.spawn()?;
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

    fn run_with_env(&mut self, env: &CliEnv, args: &[&str]) -> Result<()> {
        self.cmd = Command::new("git");
        self.with_env(env);
        self.run(args)
    }

    fn run(&mut self, args: &[&str]) -> Result<()> {
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
        let git_dir = work_tree.join(DIR_NAME);
        Self { git_dir, work_tree }
    }
}

enum CliAction {
    Init,
    #[cfg(target_os = "windows")]
    HideDotfile(String),
    Clone(String),
    RunGit(Vec<String>),
    RunLazyGit(Option<Vec<String>>),
}

fn help(code: i32) -> ! {
    let msg = "Usage: dfm <Flag> or <Commands>

Flags:
  --init                  Initialize a new dotfile repository
  --clone <url>           Clone an existing dotfile repository
  --hide-dotfile <dir>    Hidden dotfiles in <dir>
  -h|--help               Show this help message

Commands:
  lz|lazy|lazygit     Launch lazygit with dotfile environment
  <git args>          Pass through to git with environment set";

    eprintln!("{msg}");
    std::process::exit(code);
}
