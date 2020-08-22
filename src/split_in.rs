use clap::ArgMatches;

use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;


pub trait SplitOut {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
}

impl<'a> SplitOut for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        let missing_repo_name = self.repo_file.repo_name.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();

        if missing_remote_repo && missing_repo_name {
            panic!("Must provide either repo_name or remote_repo in your repofile");
        }

        if missing_include && missing_include_as {
            panic!("Must provide either include or include_as in your repofile");
        }

        if missing_repo_name && !missing_remote_repo {
            self.repo_file.repo_name = Some(try_get_repo_name_from_remote_repo(
                self.repo_file.remote_repo.clone().unwrap()
            ));
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
        .verify_dependencies()
        .validate_repo_file();
        // .save_current_dir()
        // .get_repository_from_current_dir()
        // .change_to_repo_root()
        // .generate_arg_strings()
        // .make_and_checkout_output_branch()
        // .filter_include()
        // .filter_exclude()
        // .filter_include_as();
}


#[cfg(test)]
mod test {
    use super::*;

}
