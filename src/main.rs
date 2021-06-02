use die::*;
use exechelper as exec_helpers;
use simple_interaction as interact;
use std::io;

mod repo_file;
mod split_out;
mod split_in;
mod topbase;
mod check;
mod git_helpers3;
mod cli;
mod verify;
mod core;
mod sync;

fn main() {
    let mgt = cli::get_cli_input();
    cli::validate_input_and_run(mgt);
}
