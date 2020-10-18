use clap::{App, ArgMatches};

mod commands;
mod repo_file;
mod split_out;
mod split_in;
mod topbase;
mod check;
mod split;
mod git_helpers;
mod exec_helpers;

fn get_cli_input<'a>() -> ArgMatches<'a> {
    let version_str = format!(
        "{} {}",
        env!("CARGO_PKG_VERSION"),
        env!("LATEST_COMMIT"),
    );
    let mut base_app = App::new(env!("CARGO_PKG_NAME"))
        .version(version_str.as_str())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"));

    base_app = base_app.subcommands(vec![
        commands::split_in(),
        commands::split_in_as(),
        commands::split_out(),
        commands::split_out_as(),
        commands::topbase(),
        commands::check(),
    ]);

    return base_app.get_matches();
}

// in debug mode, use panic so we get a stack trace
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! die {
    () => (::std::process::exit(1));
    ($x:expr; $($y:expr),+) => ({
        panic!($($y),+);
    });
    ($($y:expr),+) => ({
        panic!($($y),+);
    });
}

// in release mode, use print so its not ugly
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! die {
    () => (::std::process::exit(1));
    ($x:expr; $($y:expr),+) => ({
        println!($($y),+);
        ::std::process::exit($x)
    });
    ($($y:expr),+) => ({
        println!($($y),+);
        ::std::process::exit(1)
    });
}


fn main() {
    let matches = get_cli_input();

    if let Some(command_name) = matches.subcommand_name() {
        commands::run_command(command_name, &matches);
    }
}
