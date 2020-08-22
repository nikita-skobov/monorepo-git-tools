use clap::{Arg, App, SubCommand, ArgMatches};

use super::split_out::run_split_out;
use super::split_in::run_split_in;

pub const REPO_FILE_ARG: &'static str = "repo-file";
pub const DRY_RUN_ARG: &'static str = "dry-run";
pub const VERBOSE_ARG: [&'static str; 2]= ["verbose", "v"];

const SPLIT_IN_STR: &'static str = "split-in";
const SPLIT_OUT_STR: &'static str = "split-out";
const SPLIT_OUT_DESCRIPTION: &'static str = "split-out descruiptuin";
const SPLIT_IN_DESCRIPTION: &'static str = "split-in decsription";

#[derive(Clone)]
pub enum CommandName {
    SplitIn,
    SplitOut,
    UnknownCommand,
}

use self::CommandName::*;

impl From<CommandName> for &'static str {
    fn from(original: CommandName) -> &'static str {
        match original {
            SplitIn => SPLIT_IN_STR,
            SplitOut => SPLIT_OUT_STR,
            UnknownCommand => "",
        }
    }
}

impl From<&str> for CommandName {
    fn from(value: &str) -> CommandName {
        match value {
            SPLIT_IN_STR => SplitIn,
            SPLIT_OUT_STR => SplitOut,
            _ => UnknownCommand,
        }
    }
}

impl CommandName {
    pub fn description(&self) -> &'static str {
        match self {
            SplitIn => SPLIT_IN_DESCRIPTION,
            SplitOut => SPLIT_OUT_DESCRIPTION,
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
        )
        .arg(
            Arg::with_name(DRY_RUN_ARG)
                .long(DRY_RUN_ARG)
                .help("Print out the steps taken, but don't actually run or change anything.")
        )
        .arg(
            Arg::with_name(VERBOSE_ARG[0])
                .long(VERBOSE_ARG[0])
                .short(VERBOSE_ARG[1])
                .help("show more detailed logs")
        );
}

pub fn split_in<'a, 'b>() -> App<'a, 'b> {
    let base = base_command(SplitIn);
    // specific arguments and stuff can go here
    return base;
}

pub fn split_out<'a, 'b>() -> App<'a, 'b> {
    let base = base_command(SplitOut);
    return base;
}

pub fn run_command(name: &str, matches: &ArgMatches) {
    let command: CommandName = name.into();
    match command {
        UnknownCommand => (),
        // it is safe to unwrap here because this function is called
        // if we know that the name subcommand exists
        SplitIn => run_split_in(matches.subcommand_matches(name).unwrap()),
        SplitOut => run_split_out(matches.subcommand_matches(name).unwrap()),
    }
}
