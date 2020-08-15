use git2::Repository;
use clap::{App, ArgMatches};

fn get_cli_input<'a>() -> ArgMatches<'a> {
    return App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .get_matches();
}

fn main() {
    let matches = get_cli_input();

    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
}
