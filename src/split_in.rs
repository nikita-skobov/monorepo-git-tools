use clap::ArgMatches;

use super::commands::INPUT_BRANCH_ARG;
use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::git_helpers;
use super::repo_file::RepoFile;

pub trait SplitOut {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
}

impl<'a> SplitOut for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        self.input_branch = match self.matches.value_of(INPUT_BRANCH_ARG) {
            None => None,
            Some(branch_name) => {
                match &self.repo {
                    None => panic!("Failed to find repo for some reason"),
                    Some(ref repo) => {
                        if ! git_helpers::branch_exists(branch_name, repo) {
                            panic!("You specified an input branch of {}, but that branch was not found", branch_name);
                        }
                        Some(branch_name.into())
                    },
                }
            },
        };

        let missing_input_branch = self.input_branch.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();

        if missing_remote_repo && missing_input_branch {
            panic!("Must provide either repo_name in your repofile, or specify a --{} argument", INPUT_BRANCH_ARG);
        }

        if missing_include && missing_include_as {
            panic!("Must provide either include or include_as in your repofile");
        }

        panic_if_array_invalid(&self.repo_file.include, true, "include");
        panic_if_array_invalid(&self.repo_file.include_as, false, "include_as");

        self
    }

    fn generate_arg_strings(mut self) -> Self {
        self
    }
}

pub fn run_split_in(matches: &ArgMatches) {
    Runner::new(matches)
        .get_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .verify_dependencies()
        .validate_repo_file();
        // .change_to_repo_root()
        // .generate_arg_strings()
        // .make_and_checkout_output_branch()
        // .filter_exclude()
        // .filter_include_as();
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo_name in your repofile, or specify a")]
    fn should_panic_if_missing_input_branch_and_remote_repo() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let repofile = RepoFile::new();
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }
}
