// use std::str::FromStr;
use gumdrop::Options;

use die::die;
use super::check::run_check;
use super::split_out::run_split_out;
use super::split_out::run_split_out_as;
use super::split_in::run_split_in;
use super::split_in::run_split_in_as;
use super::topbase::run_topbase;

// TODO: implement way to use
// --rebase
// --rebase branchname
// --rebase --other-args
// ...
// #[derive(Debug)]
// pub struct OptionalOption<T: From<String>> {
//     val: Option<T>,
// }

// impl<T: From<String>> FromStr for OptionalOption<T> {
//     type Err = String;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let s_string = s.to_string();
//         let o = OptionalOption {
//             val: Some(s_string.into())
//         };

//         Ok(o)
//     }
// }

#[derive(Debug, Options)]
pub struct MgtCommandCheck {
    // flags
    pub all: bool,
    pub local: bool,
    pub recursive: bool,
    pub remote: bool,
    pub help: bool,

    // options
    #[options(meta = "BRANCH-NAME")]
    pub local_branch: Option<String>,
    #[options(short = "b", meta = "BRANCH-NAME")]
    pub remote_branch: Option<String>,

    // positional arg: repo_file
    // (its a vec to appease gumdrop cli parser
    // but really itll be one string)
    #[options(free)]
    pub repo_file: Vec<String>,
}

#[derive(Debug, Options)]
pub struct MgtCommandTopbase {
    #[options(free)]
    pub base_or_top: Vec<String>,

    pub dry_run: bool,
    pub verbose: bool,
    pub help: bool,
}

#[derive(Debug, Options)]
pub struct MgtCommandSplit {
    #[options(short = "g", long = "gen-repo-file")]
    pub generate_repo_file: bool,
    pub verbose: bool,
    pub dry_run: bool,
    pub help: bool,


    pub input_branch: Option<String>,
    pub num_commits: Option<u32>,
    #[options(short = "o")]
    pub output_branch: Option<String>,

    #[options(no_long, short = "r")]
    pub rebase_flag: bool,
    pub rebase: Option<String>,

    #[options(no_long, short = "t")]
    pub topbase_flag: bool,
    pub topbase: Option<String>,

    #[options(long = "as")]
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
pub enum MgtSubcommands {
    Help(MgtCommandHelp),

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

    println!("USAGE:\n{}{} {}\n",
        space,
        cmd_name,
        usage_line
    );
    let sub_usage = mgt_opts.as_ref().format_sub_usage_string_sensible();
    println!("{}", sub_usage);
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

// TODO: use optional args
// pub fn get_cli_input_with_retries(args: Option<Vec<String>>) -> Result<Mgt, gumdrop::Error> {
//     let mut args = match args {
//         Some(v) => v,
//         None => ::std::env::args().collect::<Vec<_>>(),
//     };

//     match <Mgt as Options>::parse_args_default(&args[1..]) {
//         Err(e) => {
//             // if its a missing argument, see if its something we can recover
//             // by checking if it can be an optional option
//             if let gumdrop::ErrorKind::MissingArgument(ref s) = e.kind {
//                 match s.as_str() {
//                     "-r" | "--rebase" => {
//                         let arg_pos = args.iter().position(|a| a == s).unwrap();
//                         args.insert(arg_pos + 1, "".into());
//                         get_cli_input_with_retries(Some(args))
//                     },
//                     _ => Err(e),
//                 }
//             } else {
//                 Err(e)
//             }
//         }
//         Ok(m) => Ok(m),
//     }
// }

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
            MgtSubcommands::Topbase(mut cmd) => {
                cmd.verbose = cmd.verbose || mgt_opts.verbose;
                cmd.dry_run = cmd.dry_run || mgt_opts.dry_run;
                run_topbase(&mut cmd);
            },

            MgtSubcommands::SplitIn(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::In);

                if cmd.topbase_flag && cmd.topbase.is_none() {
                    cmd.topbase = Some("".into());
                }
                if cmd.rebase_flag && cmd.rebase.is_none() {
                    cmd.rebase = Some("".into())
                }
                if cmd.rebase.is_some() && cmd.topbase.is_some() {
                    die!("Cannot use both --topbase and --rebase");
                }

                run_split_in(cmd);
            },
            MgtSubcommands::SplitInAs(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::In);

                if cmd.topbase_flag && cmd.topbase.is_none() {
                    cmd.topbase = Some("".into());
                }
                if cmd.rebase_flag && cmd.rebase.is_none() {
                    cmd.rebase = Some("".into())
                }
                if cmd.rebase.is_some() && cmd.topbase.is_some() {
                    die!("Cannot use both --topbase and --rebase");
                }

                run_split_in_as(cmd);
            },

            MgtSubcommands::SplitOut(ref mut cmd) => {
                cmd.verbose = mgt_opts.verbose || cmd.verbose;
                cmd.dry_run = mgt_opts.dry_run || cmd.dry_run;
                cmd.direction = Some(Direction::Out);

                if cmd.topbase_flag && cmd.topbase.is_none() {
                    cmd.topbase = Some("".into());
                }
                if cmd.rebase_flag && cmd.rebase.is_none() {
                    cmd.rebase = Some("".into())
                }
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
            }
        },
    }
}
