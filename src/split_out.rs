use clap::ArgMatches;
use super::commands::REPO_FILE_ARG;
use super::repo_file;
use super::repo_file::RepoFile;

pub fn validate_repo_file(matches: &ArgMatches, repofile: &mut RepoFile) {

}

pub fn run_split_out(matches: &ArgMatches) {
    // safe to unwrap because repo_file is a required argument
    let repo_file_name = matches.value_of(REPO_FILE_ARG).unwrap();
    println!("repo file: {}", repo_file_name);

    let mut repofile = repo_file::parse_repo_file(repo_file_name);
    // we validate the fields of the repo file
    // according to what split_out command wants it to be
    validate_repo_file(matches, &mut repofile);
}
