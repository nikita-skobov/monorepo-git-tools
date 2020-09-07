// this file defines the base behavior or splitting
// and running a split-X command
use std::env;
use std::path::PathBuf;
use std::path::MAIN_SEPARATOR;
use clap::ArgMatches;

use super::commands::REPO_FILE_ARG;
use super::commands::DRY_RUN_ARG;
use super::commands::VERBOSE_ARG;
use super::commands::REBASE_ARG;
use super::commands::TOPBASE_ARG;
use super::commands::OUTPUT_BRANCH_ARG;
use super::repo_file;
use super::repo_file::RepoFile;
use super::git_helpers;
use super::exec_helpers;

pub struct Runner<'a> {
    pub matches: &'a ArgMatches<'a>,
    pub current_dir: PathBuf,
    // log prefix
    pub log_p: &'static str,
    pub dry_run: bool,
    pub verbose: bool,
    pub should_rebase: bool,
    pub should_topbase: bool,
    pub repo_file: RepoFile,
    pub repo_root_dir: PathBuf,
    pub repo_original_ref: Option<String>,
    pub repo: Option<git2::Repository>,
    pub input_branch: Option<String>,
    pub output_branch: Option<String>,
    pub include_arg_str: Option<String>,
    pub include_as_arg_str: Option<String>,
    pub exclude_arg_str: Option<String>,
}

impl<'a> Runner<'a> {
    pub fn new(matches: &'a ArgMatches) -> Runner<'a> {
        let is_verbose = matches.is_present(VERBOSE_ARG[0]);
        let is_dry_run = matches.is_present(DRY_RUN_ARG[0]);
        let is_rebase = matches.is_present(REBASE_ARG[0]);
        let is_topbase = matches.is_present(TOPBASE_ARG[0]);
        let output_branch = matches.value_of(OUTPUT_BRANCH_ARG[0]);
        Runner {
            matches: matches,
            dry_run: is_dry_run,
            verbose: is_verbose,
            should_rebase: is_rebase,
            should_topbase: is_topbase,
            repo_file: RepoFile::new(),
            repo_original_ref: None,
            current_dir: PathBuf::new(),
            repo: None,
            repo_root_dir: PathBuf::new(),
            include_arg_str: None,
            include_as_arg_str: None,
            exclude_arg_str: None,
            log_p: if is_dry_run { "   # " } else { "" },
            input_branch: None,
            output_branch: if let Some(branch_name) = output_branch {
                Some(branch_name.into())
            } else {
                None
            }
        }
    }

    // get the current ref that this git repo is pointing to
    // save it for later
    pub fn save_current_ref(mut self) -> Self {
        self.repo_original_ref = match self.repo {
            Some(ref repo) => git_helpers::get_current_ref(repo),
            None => None,
        };
        self
    }

    pub fn make_and_checkout_orphan_branch(mut self, orphan_branch: &str) -> Self {
        if self.dry_run {
            println!("git checkout --orphan {}", orphan_branch);
            println!("git rm -rf . > /dev/null");
            return self;
        }

        match self.repo {
            Some(ref r) => {
                let success = git_helpers::make_orphan_branch_and_checkout(
                    orphan_branch,
                    r,
                ).is_ok();
                if ! success {
                    panic!("Failed to checkout orphan branch");
                }
                // on a new orphan branch our existing files appear in the stage
                // we need to essentially do "git rm -rf ."
                let success = git_helpers::remove_index_and_files(r).is_ok();
                if ! success {
                    panic!("Failed to remove git indexed files after making orphan");
                }
            },
            _ => panic!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out orphan branch {}", self.log_p, orphan_branch);
        }

        self
    }

    pub fn populate_empty_branch_with_remote_commits(self) -> Self {
        let remote_repo = self.repo_file.remote_repo.clone();
        let remote_branch: Option<&str> = match &self.repo_file.remote_branch {
            Some(branch_name) => Some(branch_name.as_str()),
            None => None,
        };

        match self.repo {
            None => panic!("Failed to find repo?"),
            Some(ref r) => {
                match (self.dry_run, &self.input_branch) {
                    (true, Some(branch_name)) => println!("git merge {}", branch_name),
                    (true, None) => println!("git pull {}", remote_repo.unwrap()),
                    (false, Some(branch_name)) => {
                        println!("{}Merging {}", self.log_p, branch_name);
                        git_helpers::merge_branches(&r, &branch_name[..], None);
                    },
                    (false, None) => {
                        println!("{}Pulling from {} {}", self.log_p, remote_repo.clone().unwrap_or("?".into()), remote_branch.clone().unwrap_or("".into()));
                        git_helpers::pull(&r, &remote_repo.unwrap()[..], remote_branch);
                    },
                };
            },
        };
        self
    }

    pub fn rebase(self) -> Self {
        let upstream_branch = match self.repo_original_ref {
            Some(ref branch) => branch,
            None => {
                println!("Failed to get repo original ref. Not going to rebase");
                return self;
            },
        };
        let upstream_branch = upstream_branch.replace("refs/heads/", "");

        if self.verbose {
            println!("rebasing onto {}", upstream_branch);
        }
        if self.dry_run {
            // since we are already on the rebase_from_branch
            // we dont need to specify that in the git command
            // the below command implies: apply rebased changes in
            // the branch we are already on
            println!("git rebase {}", upstream_branch);
            return self
        }

        let args = [
            "git", "rebase", upstream_branch.as_str(),
        ];
        match exec_helpers::execute(&args) {
            Err(e) => println!("Failed to rebase: {}", e),
            Ok(_) => (),
        };
        self
    }

    pub fn get_repo_file(mut self) -> Self {
        let repo_file_name = self.matches.value_of(REPO_FILE_ARG).unwrap();
        self.repo_file = repo_file::parse_repo_file(repo_file_name);
        if self.verbose {
            println!("{}got repo file: {}", self.log_p, repo_file_name);
        }
        self
    }

    pub fn save_current_dir(mut self) -> Self {
        // save this for later, as well as to find the repository
        self.current_dir = match env::current_dir() {
            Ok(pathbuf) => pathbuf,
            Err(_) => panic!("Failed to find your current directory. Cannot proceed"),
        };
        if self.verbose {
            println!("{}saving current dir to return to later: {}", self.log_p, self.current_dir.display());
        }
        self
    }
    pub fn get_repository_from_current_dir(mut self) -> Self {
        let (repo, repo_path) = git_helpers::get_repository_and_root_directory(&self.current_dir);
        self.repo = Some(repo);
        self.repo_root_dir = repo_path;
        if self.verbose {
            println!("{}found repo path: {}", self.log_p, self.repo_root_dir.display());
        }
        self
    }
    pub fn change_to_repo_root(self) -> Self {
        if self.dry_run {
            println!("cd {}", self.repo_root_dir.display());
            return self;
        }
        if ! changed_to_repo_root(&self.repo_root_dir) {
            panic!("Failed to change to repository root: {:?}", &self.repo_root_dir);
        }
        if self.verbose {
            println!("{}changed to repository root {}", self.log_p, self.repo_root_dir.display());
        }
        self
    }

    // panic if all dependencies are not met
    pub fn verify_dependencies(self) -> Self {
        if ! exec_helpers::executed_successfully(&["git", "--version"]) {
            panic!("Failed to run. Missing dependency 'git'");
        }
        if ! exec_helpers::executed_successfully(&["git", "filter-repo", "--version"]) {
            panic!("Failed to run. Missing dependency 'git-filter-repo'");
        }
        self
    }
    pub fn run_filter(self, arg_vec: Vec<&str>, verbose_log: &str) -> Self {
        if self.dry_run {
            println!("{}", arg_vec.join(" "));
            return self
        }
        if self.verbose {
            println!("{}", verbose_log);
        }
        if ! exec_helpers::executed_successfully(&arg_vec) {
            panic!("Failed to execute: \"{}\"", arg_vec.join(" "));
        }

        self
    }

    pub fn filter_include(self) -> Self {
        if self.include_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.output_branch.clone().unwrap();
        let include_arg_str_opt = self.include_arg_str.clone();
        let include_arg_str = include_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            include_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering include")
    }
    pub fn filter_include_as(self) -> Self {
        if self.include_as_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.output_branch.clone().unwrap();
        let include_as_arg_str_opt = self.include_as_arg_str.clone();
        let include_as_arg_str = include_as_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            include_as_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering include_as")
    }
    pub fn filter_exclude(self) -> Self {
        if self.exclude_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.output_branch.clone().unwrap();
        let exclude_arg_str_opt = self.exclude_arg_str.clone();
        let exclude_arg_str = exclude_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            exclude_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering exclude")
    }
}

pub fn generate_filter_arg_vec<'a>(
    arg_str: &'a str,
    output_branch: &'a str
) -> Vec<&'a str> {
    let mut arg_vec = vec!["git", "filter-repo"];
    for arg in arg_str.split_whitespace() {
        arg_vec.push(arg);
    }
    arg_vec.push("--refs");
    arg_vec.push(&output_branch);
    arg_vec.push("--force");

    arg_vec
}

fn get_string_after_last_slash(s: String) -> String {
    let mut pieces = s.rsplit(MAIN_SEPARATOR);
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}

fn get_string_before_first_dot(s: String) -> String {
    let mut pieces = s.split('.');
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}

pub fn is_valid_remote_repo(remote_repo: &String) -> bool {
    // TODO:
    // need to check for if it matches a regex like a server ip
    // like 192.168.1.1, or user@server.com:/gitpath
    return remote_repo.starts_with("ssh://") ||
    remote_repo.starts_with("git://") ||
    remote_repo.starts_with("http://") ||
    remote_repo.starts_with("https://") ||
    remote_repo.starts_with("ftp://") ||
    remote_repo.starts_with("sftp://") ||
    remote_repo.starts_with("file://") ||
    remote_repo.starts_with(".") ||
    remote_repo.starts_with("/");
}

// try to parse the remote repo
pub fn try_get_repo_name_from_remote_repo(remote_repo: String) -> String {
    let mut out_str = remote_repo.clone().trim_end().to_string();
    if !is_valid_remote_repo(&remote_repo) {
        out_str = "".into();
    }
    if out_str.ends_with(MAIN_SEPARATOR) {
        out_str.pop();
    }
    if !out_str.contains(MAIN_SEPARATOR) {
        out_str = "".into();
    }
    out_str = get_string_after_last_slash(out_str);
    out_str = get_string_before_first_dot(out_str);

    if out_str == "" {
        panic!("Failed to parse repo_name from remote_repo: {}", remote_repo);
    }

    return out_str;
}

// works for include, or include_as
// the variable is valid if it is a single item,
// or if it is multiple items, it is valid if it has an even length
pub fn include_var_valid(var: &Vec<String>, can_be_single: bool) -> bool {
    let vlen = var.len();
    if vlen == 1 && can_be_single {
        return true;
    }
    if vlen >= 1 && vlen % 2 == 0 {
        return true;
    }
    return false;
}

pub fn panic_if_array_invalid(var: &Option<Vec<String>>, can_be_single: bool, varname: &str) {
    match var {
        Some(v) => {
            if ! include_var_valid(&v, can_be_single) {
                panic!("{} is invalid. Must be either a single string, or an even length array of strings", varname);
            }
        },
        _ => (),
    };
}

pub fn changed_to_repo_root(repo_root: &PathBuf) -> bool {
    match env::set_current_dir(repo_root) {
        Ok(_) => true,
        Err(_) => false,
    }
}


// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     #[should_panic(expected = "Must provide either repo")]
//     fn should_panic_if_no_repo_name_or_remote_repo() {
//         let mut repofile = RepoFile::new();
//         let argmatches = ArgMatches::new();
//         validate_repo_file(&argmatches, &mut repofile);
//     }
// }
