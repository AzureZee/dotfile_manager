use std::io::Result;
use std::path::PathBuf;
use std::process::Command;

const DIR_NAME: &str = ".cfg";

fn main() -> Result<()> {
    let args = args_parse();
    let Some(args) = args else {
        // TODO: help message
        help_message();
        std::process::exit(1);
    };

    run(args, CliEnv::new())
}

fn args_parse() -> Option<CliArgs> {
    let mut args = std::env::args().skip(1).peekable();
    let arg = args.peek()?;
    let action = match arg.as_str() {
        "--init" => CliArgs::Init,

        "--clone" => {
            args.next();
            let url = args.next()?;
            CliArgs::Clone(url)
        }

        "lz" | "lazy" | "lazygit" => {
            args.next();
            let args: Option<Vec<String>> = args.peek().is_some().then_some(args.collect());
            CliArgs::LazyGit(args)
        }

        _ => {
            let args: Vec<String> = args.collect();
            CliArgs::Git(args)
        }
    };
    Some(action)
}

fn run(cli_args: CliArgs, env: CliEnv) -> Result<()> {
    let git_dir = unsafe { env.git_dir.to_str().unwrap_unchecked() };
    let mut cli = Cli::new();

    match cli_args {
        CliArgs::Init => {
            cli.run_git(&["init", "--bare", git_dir])?;

            let gitignore = env.work_tree.join(".gitignore");
            std::fs::write(&gitignore, format!("{}\n", DIR_NAME))?;

            let array: [&[&str]; 2] = [
                &["add", unsafe { gitignore.to_str().unwrap_unchecked() }],
                &["commit", "-m", &format!("ignore {}", DIR_NAME)],
            ];
            for args in array {
                cli.with_env_run(&env, args)?;
            }
        }

        CliArgs::Clone(url) => {
            cli.run_git(&["clone", "--bare", url.as_str(), git_dir])?;

            let array: [&[&str]; 5] = [
                &["checkout"],
                &[
                    "config",
                    "--local",
                    "remote.origin.fetch",
                    "+refs/heads/*:refs/remotes/origin/*",
                ],
                &["fetch", "origin"],
                &["branch", "-u", "origin/main"],
                &["config", "--local", "status.showUntrackedFiles", "no"],
            ];
            for args in array {
                cli.with_env_run(&env, args)?;
            }
        }
        CliArgs::Git(args) => {
            cli.with_env(&env);
            cli.cmd.args(args);
            cli.cmd.spawn()?.wait().map(|_| ())?;
        }

        CliArgs::LazyGit(args) => {
            cli.cmd = Command::new("lazygit");
            cli.with_env(&env);
            if let Some(args) = args {
                cli.cmd.args(args);
            }
            cli.cmd.spawn()?.wait().map(|_| ())?;
        }
    }
    Ok(())
}

impl Cli {
    fn new() -> Self {
        let cmd = Command::new("git");
        Self { cmd }
    }

    fn with_env(&mut self, env: &CliEnv) {
        self.cmd
            .env("GIT_DIR", &env.git_dir)
            .env("GIT_WORK_TREE", &env.work_tree);
    }

    fn with_env_run(&mut self, env: &CliEnv, args: &[&str]) -> Result<()> {
        self.cmd = Command::new("git");
        self.with_env(env);
        self.run_git(args)
    }

    fn run_git(&mut self, args: &[&str]) -> Result<()> {
        self.cmd.args(args);
        self.cmd.spawn()?.wait().map(|_| ())
    }
}

struct Cli {
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

enum CliArgs {
    Init,
    Clone(String),
    Git(Vec<String>),
    LazyGit(Option<Vec<String>>),
}

fn help_message() {}
