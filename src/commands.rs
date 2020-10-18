use clap::{Arg, App, SubCommand, ArgMatches};

use super::split_out::run_split_out;
use super::split_out::run_split_out_as;
use super::split_in::run_split_in;
use super::split_in::run_split_in_as;
use super::topbase::run_topbase;
use super::check::run_check;

pub const INPUT_BRANCH_ARG: &'static str = "input-branch";
pub const INPUT_BRANCH_NAME: &'static str = "branch-name";
pub const OUTPUT_BRANCH_ARG: [&'static str; 2] = ["output-branch", "o"];
pub const OUTPUT_BRANCH_NAME: &'static str = "branch-name";
pub const REPO_FILE_ARG: &'static str = "repo-file";
pub const REPO_URI_ARG: &'static str = "git-repo-uri";
pub const AS_SUBDIR_ARG: &'static str = "as";
pub const AS_SUBDIR_ARG_NAME: &'static str = "subdirectory";
pub const DRY_RUN_ARG: [&'static str; 2] = ["dry-run", "d"];
pub const VERBOSE_ARG: [&'static str; 2] = ["verbose", "v"];
pub const REBASE_ARG: [&'static str; 2] = ["rebase", "r"];
pub const TOPBASE_ARG: [&'static str; 2] = ["topbase", "t"];
pub const TOPBASE_CMD_TOP: &'static str = "top";
pub const TOPBASE_CMD_BASE: &'static str = "base";
pub const LOCAL_ARG: [&'static str; 2] = ["local", "l"];
pub const REMOTE_ARG: [&'static str; 2] = ["remote", "m"];
pub const REMOTE_BRANCH_ARG: [&'static str; 2] = ["remote-branch", "b"];
pub const LOCAL_BRANCH_ARG: &'static str = "local-branch";
pub const RECURSIVE_ARG: [&'static str; 2] = ["recursive", "r"];
pub const ALL_ARG: [&'static str; 2] = ["all", "a"];

const SPLIT_IN_STR: &'static str = "split-in";
const SPLIT_IN_AS_STR: &'static str = "split-in-as";
const SPLIT_OUT_STR: &'static str = "split-out";
const SPLIT_OUT_AS_STR: &'static str = "split-out-as";
const TOPBASE_CMD_STR: &'static str = "topbase";
const CHECK_CMD_STR: &'static str = "check";
const SPLIT_OUT_DESCRIPTION: &'static str = "rewrite this repository history onto a new branch such that it only contains certain paths according to a repo-file";
const SPLIT_IN_DESCRIPTION: &'static str = "fetch and rewrite a remote repository's history onto a new branch such that it only contains certain paths according to a repo-file";
const SPLIT_IN_AS_DESCRIPTION: &'static str = "fetch the entirety of a remote repository and place it in a subdirectory of this repository";
const SPLIT_OUT_AS_DESCRIPTION: &'static str = "make a new repository (via a branch) that only contains commits that are part of a subdirectory";
const TOPBASE_CMD_DESCRIPTION: &'static str = "rebases top branch onto bottom branch keeping only the first commits until it finds a commit from top where all blobs exist in the bottom branch.";
const CHECK_CMD_DESCRIPTION: &'static str = "check if remote has commits not present in local or vice versa";
const REPO_FILE_DESCRIPTION: &'static str = "path to file that contains instructions of how to split a repository";
const REPO_URI_DESCRIPTION: &'static str = "a valid git url of the repository to split in";
const AS_SUBDIR_DESCRIPTION: &'static str = "path relative to root of the local repository that will contain the entire repository being split";
const REBASE_DESCRIPTION: &'static str = "after generating a branch with rewritten history, rebase that branch such that it can be fast forwarded back into the comparison branch. For split-in, the comparison branch is the branch you started on. For split-out, the comparison branch is the remote branch";
const TOPBASE_DESCRIPTION: &'static str = "like rebase, but it finds a fork point to only take the top commits from the created branch that dont exist in your starting branch";
const TOPBASE_TOP_DESCRIPTION: &'static str = "the branch that will be rebased. defaults to current branch";
const TOPBASE_BASE_DESCRIPTION: &'static str = "the branch to rebase onto.";
const LOCAL_ARG_DESCRIPTION: &'static str = "check if the local branch has commits not present in remote";
const REMOTE_ARG_DESCRIPTION: &'static str = "check if the remote has commits not present in this local branch. This is the default";
const REMOTE_BRANCH_ARG_DESCRIPTION: &'static str = "check updates to/from a specific remote branch instead of what's in the repo file";
const LOCAL_BRANCH_ARG_DESCRIPTION: &'static str = "check updates to/from a specific local branch instead of the current HEAD";

#[derive(Clone)]
pub enum CommandName {
    SplitInAs,
    SplitIn,
    SplitOut,
    SplitOutAs,
    Topbase,
    Check,
    UnknownCommand,
}

use self::CommandName::*;

impl From<CommandName> for &'static str {
    fn from(original: CommandName) -> &'static str {
        match original {
            SplitInAs => SPLIT_IN_AS_STR,
            SplitIn => SPLIT_IN_STR,
            SplitOut => SPLIT_OUT_STR,
            SplitOutAs => SPLIT_OUT_AS_STR,
            Topbase => TOPBASE_CMD_STR,
            Check => CHECK_CMD_STR,
            UnknownCommand => "",
        }
    }
}

impl From<&str> for CommandName {
    fn from(value: &str) -> CommandName {
        match value {
            SPLIT_IN_AS_STR => SplitInAs,
            SPLIT_IN_STR => SplitIn,
            SPLIT_OUT_STR => SplitOut,
            SPLIT_OUT_AS_STR => SplitOutAs,
            TOPBASE_CMD_STR => Topbase,
            CHECK_CMD_STR => Check,
            _ => UnknownCommand,
        }
    }
}

impl CommandName {
    pub fn description(&self) -> &'static str {
        match self {
            SplitInAs => SPLIT_IN_AS_DESCRIPTION,
            SplitIn => SPLIT_IN_DESCRIPTION,
            SplitOut => SPLIT_OUT_DESCRIPTION,
            SplitOutAs => SPLIT_OUT_AS_DESCRIPTION,
            Topbase => TOPBASE_CMD_DESCRIPTION,
            Check => CHECK_CMD_DESCRIPTION,
            _ => "",
        }
    }
}

fn base_command<'a, 'b>(cmd: CommandName) -> App<'a, 'b> {
    let name = cmd.clone().into();
    return SubCommand::with_name(name)
        .about(cmd.description())
        .arg(
            Arg::with_name(REPO_FILE_ARG)
                .required(true)
                .help(REPO_FILE_DESCRIPTION)
        )
        .arg(
            Arg::with_name(DRY_RUN_ARG[0])
                .long(DRY_RUN_ARG[0])
                .short(DRY_RUN_ARG[1])
                .help("Print out the steps taken, but don't actually run or change anything.")
        )
        .arg(
            Arg::with_name(VERBOSE_ARG[0])
                .long(VERBOSE_ARG[0])
                .short(VERBOSE_ARG[1])
                .help("show more detailed logs")
        )
        .arg(
            Arg::with_name(REBASE_ARG[0])
                .long(REBASE_ARG[0])
                .short(REBASE_ARG[1])
                .takes_value(true)
                .default_value("")
                .help(REBASE_DESCRIPTION)
        )
        .arg(
            Arg::with_name(TOPBASE_ARG[0])
                .long(TOPBASE_ARG[0])
                .short(TOPBASE_ARG[1])
                .takes_value(true)
                .default_value("")
                .help(TOPBASE_DESCRIPTION)
        )
        .arg(
            Arg::with_name(OUTPUT_BRANCH_ARG[0])
                .long(OUTPUT_BRANCH_ARG[0])
                .short(OUTPUT_BRANCH_ARG[1])
                .takes_value(true)
                .value_name(OUTPUT_BRANCH_NAME)
                .help("name of branch that will be created with new split history")
        );
}

pub fn split_in<'a, 'b>() -> App<'a, 'b> {
    // split in has specific arguments in addition to base
    base_command(SplitIn)
        .arg(
            Arg::with_name(INPUT_BRANCH_ARG)
                .long(INPUT_BRANCH_ARG)
                .takes_value(true)
                .value_name(INPUT_BRANCH_NAME)
                .help("split in from a local branch in this repository")
        )
}

pub fn split_in_as<'a, 'b>() -> App<'a, 'b> {
    // split in as has specific arguments in addition to base
    let cmd = SplitInAs;
    let name = cmd.clone().into();
    return SubCommand::with_name(name)
        .about(cmd.description())
        .arg(
            Arg::with_name(REPO_URI_ARG)
                .required(true)
                .help(REPO_URI_DESCRIPTION)
        )
        .arg(
            Arg::with_name(REBASE_ARG[0])
                .long(REBASE_ARG[0])
                .short(REBASE_ARG[1])
                .help(REBASE_DESCRIPTION)
                .takes_value(true)
                .default_value("")
        )
        // TODO: should remove topbase from split-in-as? i dont think it makes sense
        .arg(
            Arg::with_name(TOPBASE_ARG[0])
                .long(TOPBASE_ARG[0])
                .short(TOPBASE_ARG[1])
                .takes_value(true)
                .default_value("")
                .help(TOPBASE_DESCRIPTION)
        )
        .arg(
            Arg::with_name(AS_SUBDIR_ARG)
                .long(AS_SUBDIR_ARG)
                .help(AS_SUBDIR_DESCRIPTION)
                .value_name(AS_SUBDIR_ARG_NAME)
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DRY_RUN_ARG[0])
                .long(DRY_RUN_ARG[0])
                .short(DRY_RUN_ARG[1])
                .help("Print out the steps taken, but don't actually run or change anything.")
        )
        .arg(
            Arg::with_name(VERBOSE_ARG[0])
                .long(VERBOSE_ARG[0])
                .short(VERBOSE_ARG[1])
                .help("show more detailed logs")
        )
        .arg(
            Arg::with_name(OUTPUT_BRANCH_ARG[0])
                .long(OUTPUT_BRANCH_ARG[0])
                .short(OUTPUT_BRANCH_ARG[1])
                .takes_value(true)
                .value_name(OUTPUT_BRANCH_NAME)
                .help("name of branch that will be created with new split history")
        );
}

pub fn split_out_as<'a, 'b>() -> App<'a, 'b> {
    // split in as has specific arguments in addition to base
    let cmd = SplitOutAs;
    let name = cmd.clone().into();
    return SubCommand::with_name(name)
        .about(cmd.description())
        .arg(
            Arg::with_name(AS_SUBDIR_ARG)
                .long(AS_SUBDIR_ARG)
                .help(AS_SUBDIR_DESCRIPTION)
                .value_name(AS_SUBDIR_ARG_NAME)
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(DRY_RUN_ARG[0])
                .long(DRY_RUN_ARG[0])
                .short(DRY_RUN_ARG[1])
                .help("Print out the steps taken, but don't actually run or change anything.")
        )
        .arg(
            Arg::with_name(VERBOSE_ARG[0])
                .long(VERBOSE_ARG[0])
                .short(VERBOSE_ARG[1])
                .help("show more detailed logs")
        )
        .arg(
            Arg::with_name(OUTPUT_BRANCH_ARG[0])
                .long(OUTPUT_BRANCH_ARG[0])
                .short(OUTPUT_BRANCH_ARG[1])
                .takes_value(true)
                .value_name(OUTPUT_BRANCH_NAME)
                .required(true)
                .help("name of branch that will be created with new split history")
        );
}

pub fn split_out<'a, 'b>() -> App<'a, 'b> {
    base_command(SplitOut)
}

pub fn topbase<'a, 'b>() -> App<'a, 'b> {
    let cmd = Topbase;
    let name = cmd.clone().into();
    return SubCommand::with_name(name)
        .about(cmd.description())
        .arg(
            Arg::with_name(DRY_RUN_ARG[0])
                .long(DRY_RUN_ARG[0])
                .short(DRY_RUN_ARG[1])
                .help("Print out the steps taken, but don't actually run or change anything.")
        )
        .arg(
            Arg::with_name(VERBOSE_ARG[0])
                .long(VERBOSE_ARG[0])
                .short(VERBOSE_ARG[1])
                .help("show more detailed logs")
        )
        .arg(
            Arg::with_name(TOPBASE_CMD_BASE)
                .required(true)
                .help(TOPBASE_BASE_DESCRIPTION)
        )
        .arg(
            Arg::with_name(TOPBASE_CMD_TOP)
                .help(TOPBASE_TOP_DESCRIPTION)
        );
}

pub fn check<'a, 'b>() ->App<'a, 'b> {
    let cmd = Check;
    let name = cmd.clone().into();

    return SubCommand::with_name(name)
        .about(cmd.description())
        .arg(
            Arg::with_name(REPO_FILE_ARG)
                .help(REPO_FILE_DESCRIPTION)
                .required(true)
        )
        .arg(
            Arg::with_name(REMOTE_ARG[0])
                .help(REMOTE_ARG_DESCRIPTION)
                .long(REMOTE_ARG[0])
                .short(REMOTE_ARG[1])
                .conflicts_with(LOCAL_ARG[0])
        )
        .arg(
            Arg::with_name(LOCAL_ARG[0])
                .help(LOCAL_ARG_DESCRIPTION)
                .long(LOCAL_ARG[0])
                .short(LOCAL_ARG[1])
                .conflicts_with(REMOTE_ARG[0])
        )
        .arg(
            Arg::with_name(REMOTE_BRANCH_ARG[0])
                .help(REMOTE_BRANCH_ARG_DESCRIPTION)
                .long(REMOTE_BRANCH_ARG[0])
                .short(REMOTE_BRANCH_ARG[1])
                .takes_value(true)
        )
        .arg(
            Arg::with_name(RECURSIVE_ARG[0])
                .long(RECURSIVE_ARG[0])
                .short(RECURSIVE_ARG[1])
                .help("if the <repo-file> is a directory, get all files in this directory recursively")
        )
        .arg(
            Arg::with_name(ALL_ARG[0])
                .long(ALL_ARG[0])
                .short(ALL_ARG[1])
                .help("if the <repo-file> is a directory, by default mgt only looks for files ending in .rf, with the --all flag, you are telling mgt to get any file it finds from the <repo-file> directory")
        )
        .arg(
            Arg::with_name(LOCAL_BRANCH_ARG)
                .help(LOCAL_BRANCH_ARG_DESCRIPTION)
                .long(LOCAL_BRANCH_ARG)
                .takes_value(true)
        );
}

pub fn run_command(name: &str, matches: &ArgMatches) {
    let command: CommandName = name.into();
    match command {
        UnknownCommand => (),
        // it is safe to unwrap here because this function is called
        // if we know that the name subcommand exists
        SplitIn => run_split_in(matches.subcommand_matches(name).unwrap()),
        SplitInAs => run_split_in_as(matches.subcommand_matches(name).unwrap()),
        SplitOut => run_split_out(matches.subcommand_matches(name).unwrap()),
        SplitOutAs => run_split_out_as(matches.subcommand_matches(name).unwrap()),
        Topbase => run_topbase(matches.subcommand_matches(name).unwrap()),
        Check => run_check(matches.subcommand_matches(name).unwrap()),
    }
}
