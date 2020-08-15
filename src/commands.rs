use clap::{Arg, App, SubCommand, ArgMatches};

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
            Arg::with_name("repo_file")
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
        SplitIn => run_split_in(matches),
        SplitOut => run_split_out(matches),
    }
}

pub fn run_split_in(matches: &ArgMatches) {

}

pub fn run_split_out(matches: &ArgMatches) {

}
