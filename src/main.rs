use std::ffi::OsStr;
use std::io::Result;
use std::path::PathBuf;
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
        "--echo-env" => CliAction::EchoEnv,

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
    let git_dir = &env.git_dir.to_string_lossy();
    let mut git = GitWrapper::new();

    match action {
        CliAction::Init => {
            git.run(["init", "--bare", git_dir])?;
            git.run_with_env(&env, NO_SHOW_UNTRACKED)?;

            let gitignore = env.work_tree.join(".gitignore");

            if !gitignore.exists() {
                std::fs::write(&gitignore, GITIGNORE_CONTENT)?;

                let steps: [&[&str]; 2] = [
                    &["add", &gitignore.to_string_lossy()],
                    &["commit", "-m", "add .gitignore"],
                ];
                for args in &steps[..] {
                    git.run_with_env(&env, *args)?;
                }
            } else {
                println!(
                    "\n`.gitignore` already exists,\nplease add follow content to your `.gitignore`.\n{}",
                    GITIGNORE_CONTENT
                )
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

        CliAction::EchoEnv => {
            println!(
                "GIT_DIR: {}\nGIT_WORK_TREE: {}",
                git_dir,
                env.work_tree.display()
            );
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
    EchoEnv,
    #[cfg(target_os = "windows")]
    HideDotfile(String, bool),
    Clone(String, Option<String>),
    RunGit(Vec<String>),
    RunLazyGit(Option<Vec<String>>),
}

fn show_help(code: i32) -> ! {
    let msg = "Usage: dfm <Flag> or <Commands>

Flags:
  --init                  Initialize a new dotfile repository in $HOME
  --clone <url> [branch]  Clone an existing dotfile repository,
                          after checkout branch(optional)
  --echo-env              Print current env.
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

const GITIGNORE_CONTENT: &str = "# ignore all
*

# white list
!.config/
!.config/**
!.bashrc
!.zshrc
!.gitconfig*
!.gitignore*
!.*profile

# black list
.cfg/
";
