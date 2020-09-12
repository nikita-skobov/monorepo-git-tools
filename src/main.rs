use clap::{App, ArgMatches};

mod commands;
mod repo_file;
mod split_out;
mod split_in;
mod topbase;
mod split;
mod git_helpers;
mod exec_helpers;

fn get_cli_input<'a>() -> ArgMatches<'a> {
    let mut base_app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"));

    base_app = base_app.subcommands(vec![
        commands::split_in(),
        commands::split_in_as(),
        commands::split_out(),
        commands::split_out_as(),
        commands::topbase(),
    ]);

    return base_app.get_matches();
}

fn main() {
    let matches = get_cli_input();

    if let Some(command_name) = matches.subcommand_name() {
        commands::run_command(command_name, &matches);
    }
}
