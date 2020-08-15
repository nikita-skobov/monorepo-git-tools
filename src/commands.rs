use clap::{Arg, App, SubCommand, ArgMatches};

const SPLIT_IN_STR: &'static str = "split-in";
const SPLIT_OUT_STR: &'static str = "split-out";

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


pub fn split_in<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name(SplitIn.into())
        .about("splits a remote repo into this local repo")
        .arg(
            Arg::with_name("repo_file")
        );
}

pub fn split_out<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name(SplitOut.into())
        .about("splits this local repo out into a subrepo")
        .arg(
            Arg::with_name("repo_file")
        );
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
