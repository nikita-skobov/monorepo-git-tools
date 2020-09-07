use clap::ArgMatches;

use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;
use super::git_helpers;

pub trait Topbase {
    fn topbase(self) -> Self;
}

pub fn run_topbase(matches: &ArgMatches) {

}
