use gumdrop::Options;

use die::die;
use super::check::run_check;
use super::difflog::run_difflog;
use super::split_out::run_split_out;
use super::split_out::run_split_out_as;
use super::split_in::run_split_in;
use super::split_in::run_split_in_as;
use super::verify::run_verify;
use super::topbase::run_topbase;
use super::topbase::ABTraversalMode;
use super::sync::run_sync;
use std::path::PathBuf;

#[derive(Debug, Options)]
pub struct MgtCommandCheck {
    // flags
    #[options(help = "if the <repo-file> is a directory, by default mgt only looks for files ending in .rf, but with the --all flag, you are telling mgt to get any file it finds from the <repo-file> directory")]
    pub all: bool,
    #[options(help = "check if the local branch has commits not present in remote")]
    pub local: bool,
    #[options(help = "if the <repo-file> is a directory, get all files in this directory recursively")]
    pub recursive: bool,
    #[options(help = "check if the remote has commits not present in this local branch. This is the default")]
    pub remote: bool,
    #[options(short = "h")]
    pub help: bool,

    // options
    #[options(meta = "BRANCH-NAME", help = "check updates to/from a specific local branch instead of the current HEAD")]
    pub local_branch: Option<String>,
    #[options(short = "b", meta = "BRANCH-NAME", help = "check updates to/from a specific remote branch instead of what's in the repo file")]
    pub remote_branch: Option<String>,

    // positional arg: repo_file
    // (its a vec to appease gumdrop cli parser
    // but really itll be one string)
    #[options(free)]
    pub repo_file: Vec<String>,
}

#[derive(Debug, Options)]
pub struct MgtCommandDifflog {
    #[options(free)]
    pub branches: Vec<String>,

    #[options(short = "m", help = "(fullbase implemented yet) Valid modes are [topbase, rewind, fullbase]. default is rewind")]
    pub traversal_mode: Option<ABTraversalMode>,

    #[options(short = "w", help = "Force specify a width to display the log. default is to use whole terminal")]
    pub term_width: Option<usize>,

    #[options(short = "h")]
    pub help: bool,
}

#[derive(Debug, Options)]
pub struct MgtCommandTopbase {
    #[options(free)]
    pub base_or_top: Vec<String>,

    #[options(help = "Print out the steps taken, but don't actually run or change anything.")]
    pub dry_run: bool,
    #[options(help = "Prints verbose information")]
    pub verbose: bool,
    #[options(short = "h")]
    pub help: bool,
}

#[derive(Debug, Options)]
pub struct MgtCommandSplit {
    #[options(short = "g", long = "gen-repo-file", help = "generate a repo file from the provided remote repo and the --as argument gets mapped to [include_as]")]
    pub generate_repo_file: bool,
    #[options(help = "Prints verbose information")]
    pub verbose: bool,
    #[options(help = "Print out the steps taken, but don't actually run or change anything.")]
    pub dry_run: bool,
    #[options(short = "h")]
    pub help: bool,

    #[options(help = "split in from a local branch in this repository")]
    pub input_branch: Option<String>,
    #[options(meta = "N", help = "when pulling from remote, limit to N commits from the current tip. This is probably only useful the first time you do a split-in")]
    pub num_commits: Option<u32>,
    #[options(short = "o", help = "name of branch that will be created with new split history")]
    pub output_branch: Option<String>,

    #[options(optional, short = "r", help = "after generating a branch with rewritten history, rebase that branch such that it can be fast forwarded back into the comparison branch. for split-in that is the branch you started on. For split-out, that is the remote branch. Optionally provide a '--rebase BRANCH-NAME' to rebase onto that branch instead of the default.")]
    pub rebase: Option<String>,

    #[options(optional, short = "t", help = "like --rebase, but it finds a fork point by stopping at the first commit that two branches have in common. This is useful as an 'update' mechanism. Optionally provide a '--topbase BRANCH-NAME' to topbase onto that branch instead of the default.")]
    pub topbase: Option<String>,

    #[options(long = "as", help = "path relative to root of the local repository that will contain the entire repository being split")]
    pub as_subdir: Option<String>,

    // for program use, not by user
    #[options(skip)]
    pub direction: Option<Direction>,

    // positional arg: repo_file
    // (its a vec to appease gumdrop cli parser
    // but really itll be one string)
    #[options(free)]
    pub repo_file: Vec<String>,
}

#[derive(Debug)]
pub enum Direction { Out, In }

#[derive(Debug, Options)]
pub struct MgtCommandHelp {}

#[derive(Debug, Options)]
pub struct MgtCommandVerify {
    #[options(short = "h")]
    pub help: bool,

    #[options(free, help = "path to your repo file")]
    pub repo_file: Vec<String>,

    #[options(help = "show full rename from src -> dest")]
    pub verbose: bool,

    #[options(help = "format the mapping nicely. implies verbose. wont work well on small terminals though")]
    pub pretty: bool,

    #[options(help = "provide a list of files to verify from stdin, one file per line. By default we get this list of files for you via:\ngit ls-tree -r HEAD --name-only --full-tree\n You can achieve the default behavior by doing:\n git ls-tree -r HEAD --name-only --full-tree | mgt verify-rf --stdin <PATH/TO/REPOFILE>")]
    pub stdin: bool,
}

#[derive(Debug, Options)]
pub struct MgtCommandSync {
    #[options(short = "h")]
    pub help: bool,

    #[options(free, help = "path to repo file(s) or a folder container repo files")]
    pub repo_files: Vec<PathBuf>,

    #[options(help = "when iterating the sync of multiple repo files, if a single one fails, do not sync the rest")]
    pub fail_fast: bool,
}

#[derive(Debug, Options)]
pub enum MgtSubcommands {
    Help(MgtCommandHelp),

    #[options(help = "Interactively sync one or more repo files between local and remote repositorie(s)")]
    Sync(MgtCommandSync),

    #[options(help = "View a log comparing two branches that have potentially unrelated history using a topbase algorithm")]
    DiffLog(MgtCommandDifflog),

    #[options(help = "check if there are changes ready to be pushed or pulled")]
    Check(MgtCommandCheck),

    #[options(help = "rebase top branch onto bottom branch but stop the rebase after the first shared commit")]
    Topbase(MgtCommandTopbase),

    #[options(help = "fetch and rewrite a remote repository's history onto a new branch according to the repo file rules")]
    SplitIn(MgtCommandSplit),

    #[options(help = "fetch and rewrite a remote repository's history onto a new branch and into the --as <subdirectory>")]
    SplitInAs(MgtCommandSplit),

    #[options(help = "create a new branch with this repository's history rewritten according to the repo file rules")]
    SplitOut(MgtCommandSplit),

    #[options(help = "create a new branch with this repository's history rewritten according to the --as <subdirectory>")]
    SplitOutAs(MgtCommandSplit),

    #[options(help = "verify your repo file before running a split operation")]
    VerifyRepoFile(MgtCommandVerify),

    #[options(help = "alias for verify-repo-file")]
    VerifyRf(MgtCommandVerify),
}

pub fn get_version_str() -> String {
    format!(
        "{} {}",
        env!("CARGO_PKG_VERSION"),
        env!("LATEST_COMMIT"),
    )
}

pub fn print_usage<A: AsRef<impl Options>>(
    mgt_opts: A,
    subcommand_name: Option<&str>,
    usage_line: Option<&str>,
) {
    let space = "    ";
    let cmd_name = subcommand_name.unwrap_or("mgt");
    let usage_line = usage_line.unwrap_or("[FLAGS] [OPTIONS] <repo_file>");

    // TODO: modify gumdrop
    // to easily print positional descriptions!
    let repo_file_desc = "    <repo-file>    path to file that contains instructions of how to split a repository";

    // TODO: modify gumdrop
    // to be able to get these descriptions without
    // hardcoding them twice!
    let (positional_desc, desc, filters) = if cmd_name.contains("split-out-as") {
        let p_desc = None;
        let desc = "create a new branch with this repository's history rewritten according to the --as <subdirectory>";
        (p_desc, desc, Some(vec!["input-branch", "gen-repo-file", "num-commits", "--rebase", "--topbase"]))
    } else if cmd_name.contains("split-out") {
        let p_desc = Some(repo_file_desc);
        let desc = "create a new branch with this repository's history rewritten according to the repo file rules";
        (p_desc, desc, Some(vec!["input-branch", "gen-repo-file", "--as", "num-commits"]))
    } else if cmd_name.contains("split-in-as") {
        let p_desc = Some("    <git-repo-uri>    a valid git url of the repository to split in");
        let desc = "fetch and rewrite a remote repository's history onto a new branch and into the --as <subdirectory>";
        (p_desc, desc, Some(vec!["input-branch"]))
    } else if cmd_name.contains("split-in") {
        let p_desc = Some(repo_file_desc);
        let desc = "fetch and rewrite a remote repository's history onto a new branch according to the repo file rules";
        (p_desc, desc, Some(vec!["gen-repo-file", "--as"]))
    } else if cmd_name.contains("topbase") {
        let p_desc = Some("    <base>    the branch to rebase onto.\n    [top]     the branch that will be rebased. defaults to current branch");
        let desc = "rebase top branch onto bottom branch but stop the rebase after the first shared commit";
        (p_desc, desc, None)
    } else if cmd_name.contains("check") {
        let p_desc = Some(repo_file_desc);
        let desc = "check if there are changes ready to be pushed or pulled";
        (p_desc, desc, None)
    } else {
        (None, "", None)
    };

    println!("{}\n", desc);

    println!("USAGE:\n{}{} {}\n",
        space,
        cmd_name,
        usage_line
    );

    let sub_usage = mgt_opts.as_ref()
        .format_sub_usage_string_with_filters(
            Some(100),
            Some(4),
            Some(4),
            filters,
        );
    println!("{}", sub_usage);

    if let Some(ref p) = positional_desc {
        println!("POSITIONAL:");
        println!("{}", p)
    }
}

pub fn print_program_usage<A: AsRef<impl Options>>(mgt_opts: A) {
    let version_str = get_version_str();
    let author = env!("CARGO_PKG_AUTHORS");
    let about = env!("CARGO_PKG_DESCRIPTION");
    let app_name = env!("CARGO_PKG_NAME");
    let space = "    ";

    println!("{} {}\n{}\n{}\n\nUSAGE:\n{}{} [SUBCOMMAND] [OPTIONS]\n",
        app_name, version_str,
        author,
        about,
        space,
        app_name
    );
    let sub_usage = mgt_opts.as_ref().format_sub_usage_string_sensible();
    println!("{}", sub_usage);

    if let Some(cmds) = mgt_opts.as_ref().self_command_list() {
        println!("Available commands:");
        println!("{}", cmds);
    }
}

#[derive(Debug, Options)]
pub struct Mgt {
    #[options(help = "Dont run anything. Just print output of what a run would do.")]
    pub dry_run: bool,
    #[options(help = "More detailed output")]
    pub verbose: bool,
    #[options(short = "h", help = "Prints help information")]
    pub help: bool,
    #[options(short = "V", help = "Prints version information")]
    pub version: bool,
    // thing: Option<String>,

    #[options(command)]
    pub command: Option<MgtSubcommands>,
}

impl AsRef<Mgt> for Mgt {
    fn as_ref(&self) -> &Mgt { self }
}
impl AsRef<MgtCommandCheck> for MgtCommandCheck {
    fn as_ref(&self) -> &MgtCommandCheck { self }
}
impl AsRef<MgtCommandTopbase> for MgtCommandTopbase {
    fn as_ref(&self) -> &MgtCommandTopbase { self }
}
impl AsRef<MgtCommandSplit> for MgtCommandSplit {
    fn as_ref(&self) -> &MgtCommandSplit { self }
}
impl AsRef<MgtCommandVerify> for MgtCommandVerify {
    fn as_ref(&self) -> &MgtCommandVerify { self }
}
impl AsRef<MgtCommandSync> for MgtCommandSync {
    fn as_ref(&self) -> &MgtCommandSync { self }
}
impl AsRef<MgtCommandDifflog> for MgtCommandDifflog {
    fn as_ref(&self) -> &MgtCommandDifflog { self }
}

impl Mgt {
    pub fn new() -> Mgt {
        Mgt {
            dry_run: false,
            verbose: false,
            help: false,
            version: false,
            command: None,
        }
    }
}

pub fn get_cli_input() -> Mgt {
    let args = ::std::env::args().collect::<Vec<_>>();
    let cli = match <Mgt as Options>::parse_args_default(&args[1..]) {
        Err(e) => {
            println!("Failed to parse cli input: {}\n", e);
            let dummy_mgt = Mgt::new();
            print_program_usage(dummy_mgt);
            std::process::exit(2);
        }
        Ok(m) => m,
    };

    if cli.version {
        println!("{}", get_version_str());
        std::process::exit(0);
    }

    if cli.command.is_none() {
        print_program_usage(&cli);
        std::process::exit(0);
    }

    let is_help = match cli.command {
        None => false,
        Some(ref cmd) => match cmd {
            // the help subcommand gets its own validation
            // and running below
            MgtSubcommands::Help(_) => false,
            MgtSubcommands::Check(c) => {
                if cli.help || c.help {
                    print_usage(&c, Some("mgt check"), None);
                    true
                } else { false }
            }
            MgtSubcommands::DiffLog(c) => {
                if cli.help || c.help {
                    print_usage(&c, Some("mgt diff-log"), None);
                    true
                } else { false}
            }
            MgtSubcommands::Topbase(t) => {
                if cli.help || t.help {
                    print_usage(&t, Some("mgt topbase"), Some("[FLAGS] <base> [top]"));
                    true
                } else { false }
            }
            MgtSubcommands::SplitIn(s) |
            MgtSubcommands::SplitInAs(s) |
            MgtSubcommands::SplitOut(s) |
            MgtSubcommands::SplitOutAs(s) => {
                let subcommand_name = cli.command_name().unwrap();
                let usage_line = if subcommand_name == "split-out-as" {
                    Some("[FLAGS] --as <subdirectory> --output-branch <branch-name>")
                } else if subcommand_name == "split-in-as" {
                    Some("[FLAGS] [OPTIONS] <git-repo-uri> --as <subdirectory>")
                } else { None };
                let subcommand_name = format!("mgt {}", subcommand_name);
                if cli.help || s.help {
                    print_usage(&s, Some(&subcommand_name), usage_line);
                    true
                } else { false }
            }
            MgtSubcommands::VerifyRepoFile(v) |
            MgtSubcommands::VerifyRf(v) => {
                if cli.help || v.help {
                    print_usage(&v, Some("mgt verify"), None);
                    true
                } else { false }
            }
            MgtSubcommands::Sync(s) => {
                if cli.help || s.help {
                    print_usage(&s, Some("mgt sync"), None);
                    true
                } else { false }
            }
        }
    };

    if is_help {
        std::process::exit(0);
    }

    // check for global program help
    // vs the above which checked for subcommand help
    if cli.help {
        print_program_usage(&cli);
        std::process::exit(0);
    }

    cli
}

/// validate the input options, and adjust as needed
/// print an error message and exit if invalid.
/// otherwise, call each commands run function
pub fn validate_input_and_run(mgt_opts: Mgt) {
    let mut mgt_opts = mgt_opts;
    match mgt_opts.command.take() {
        None => (),
        Some(mut command) => match command {
            MgtSubcommands::Help(_) => {
                // TODO: print help for the specific command
                print_program_usage(&mgt_opts);
                std::process::exit(0);
            },
            MgtSubcommands::Check(mut cmd) => {
                if cmd.remote && cmd.local {
                    die!("--remote cannot be used with --local");
                }
                run_check(&mut cmd);
            },
            MgtSubcommands::DiffLog(mut cmd) => {
                if cmd.branches.len() != 2 {
                    die!("Must provide exactly two branches to compare. You provided:\n{:?}", cmd.branches);
                }
                run_difflog(&mut cmd);
            }
            MgtSubcommands::Topbase(mut cmd) => {
                cmd.verbose = cmd.verbose || mgt_opts.verbose;
                cmd.dry_run = cmd.dry_run || mgt_opts.dry_run;
                run_topbase(&mut cmd);
            },

            MgtSubcommands::SplitIn(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::In);

                if cmd.rebase.is_some() && cmd.topbase.is_some() {
                    die!("Cannot use both --topbase and --rebase");
                }

                run_split_in(cmd);
            },
            MgtSubcommands::SplitInAs(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::In);

                if cmd.rebase.is_some() && cmd.topbase.is_some() {
                    die!("Cannot use both --topbase and --rebase");
                }

                run_split_in_as(cmd);
            },

            MgtSubcommands::SplitOut(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::Out);

                if cmd.rebase.is_some() && cmd.topbase.is_some() {
                    die!("Cannot use both --topbase and --rebase");
                }
                run_split_out(cmd);
            },
            MgtSubcommands::SplitOutAs(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::Out);
                run_split_out_as(cmd);
            },
            MgtSubcommands::VerifyRepoFile(ref mut cmd) |
            MgtSubcommands::VerifyRf(ref mut cmd) => {
                run_verify(cmd);
            }
            MgtSubcommands::Sync(ref mut cmd) => {
                run_sync(cmd);
            }
        },
    }
}
