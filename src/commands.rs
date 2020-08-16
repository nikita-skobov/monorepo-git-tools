use clap::{Arg, App, SubCommand, ArgMatches};

use super::repo_file;
use super::split_out::run_split_out;

pub const REPO_FILE_ARG: &'static str = "repo_file";

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

pub fn run_split_in(matches: &ArgMatches) {

}
