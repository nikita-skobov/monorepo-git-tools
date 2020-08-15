use clap::{Arg, App, SubCommand};

pub fn split_in<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("split-in")
        .about("splits a remote repo into this local repo")
        .arg(
            Arg::with_name("repo_file")
        );
}

pub fn split_out<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("split-out")
        .about("splits this local repo out into a subrepo")
        .arg(
            Arg::with_name("repo_file")
        );
}
