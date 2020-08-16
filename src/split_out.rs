use clap::ArgMatches;
use super::commands::REPO_FILE_ARG;
use super::repo_file;
use super::repo_file::RepoFile;

// try to parse the remote repo
pub fn try_get_repo_name_from_remote_repo(remote_repo: String) -> String {
    panic!("Failed to parse repo_name from remote_repo: {}", remote_repo);
}

pub fn validate_repo_file(matches: &ArgMatches, repofile: &mut RepoFile) {
    let missing_repo_name = repofile.repo_name.is_none();
    let missing_remote_repo = repofile.remote_repo.is_none();
    let missing_include_as = repofile.include_as.is_none();
    let missing_include = repofile.include.is_none();

    if missing_remote_repo && missing_repo_name {
        panic!("Must provide either repo_name or remote_repo in your repofile");
    }
    if missing_include && missing_include_as {
        panic!("Must provide either include or include_as in your repofile");
    }

    if missing_repo_name && !missing_remote_repo {
        repofile.repo_name = Some(try_get_repo_name_from_remote_repo(
            repofile.remote_repo.clone().unwrap()
        ));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo")]
    fn should_panic_if_no_repo_name_or_remote_repo() {
        let mut repofile = RepoFile::new();
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    #[should_panic(expected = "Must provide either include")]
    fn should_panic_if_no_include_or_include_as() {
        let mut repofile = RepoFile::new();
        repofile.repo_name = Some("reponame".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    #[should_panic(expected = "Failed to parse repo_name from remote_repo")]
    fn should_panic_if_failed_to_parse_repo_name_from_remote_repo() {
        let mut repofile = RepoFile::new();
        repofile.include = Some(vec!["sdsa".into()]);
        repofile.remote_repo = Some("badurl".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn should_parse_repo_name_from_remote_repo_if_valid() {
        let mut repofile = RepoFile::new();
        repofile.remote_repo = Some("./path/to/reponame".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
        assert_eq!(repofile.repo_name.unwrap(), "reponame".to_string());
    }
}
