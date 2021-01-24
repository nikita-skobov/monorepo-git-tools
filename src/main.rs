use die::*;

mod repo_file;
mod split_out;
mod split_in;
mod topbase;
mod check;
use exechelper as exec_helpers;
mod git_helpers3;
mod cli;
mod core;

fn main() {
    let mgt = cli::get_cli_input();
    cli::validate_input_and_run(mgt);
}
