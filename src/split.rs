// this file defines the base behavior or splitting
// and running a split-X command
use std::env;
use std::path::PathBuf;
use std::path::MAIN_SEPARATOR;
use clap::ArgMatches;
use git_url_parse::GitUrl;

use super::commands::REPO_FILE_ARG;
use super::commands::DRY_RUN_ARG;
use super::commands::VERBOSE_ARG;
use super::commands::REBASE_ARG;
use super::commands::TOPBASE_ARG;
use super::commands::OUTPUT_BRANCH_ARG;
use super::commands::NUM_COMMITS_ARG;
use super::repo_file;
use super::repo_file::RepoFile;
use super::git_helpers;
use super::git_helpers3;
use super::exec_helpers;
use super::die;

pub struct Runner<'a> {
    pub repo_file_path: Option<&'a str>,
    pub matches: &'a ArgMatches<'a>,
    pub current_dir: PathBuf,
    // log prefix
    pub log_p: &'static str,
    pub dry_run: bool,
    pub verbose: bool,
    pub should_rebase: bool,
    pub should_topbase: bool,
    pub topbase_add_label: bool,
    pub repo_file: RepoFile,
    pub repo_root_dir: PathBuf,
    pub topbase_top_ref: Option<String>,
    pub repo_original_ref: Option<String>,
    pub repo: Option<git2::Repository>,
    pub input_branch: Option<String>,
    pub output_branch: Option<String>,
    pub include_arg_str: Option<Vec<String>>,
    pub include_as_arg_str: Option<Vec<String>>,
    pub exclude_arg_str: Option<Vec<String>>,
    // only relevant to split-in
    pub num_commits: Option<u32>,
    pub status: i32,
}

impl<'a> Runner<'a> {
    pub fn new(matches: &'a ArgMatches) -> Runner<'a> {
        let is_verbose = matches.is_present(VERBOSE_ARG[0]);
        let is_dry_run = matches.is_present(DRY_RUN_ARG[0]);
        let is_rebase = matches.occurrences_of(REBASE_ARG[0]) > 0;
        let is_topbase = matches.occurrences_of(TOPBASE_ARG[0]) > 0;
        let output_branch = matches.value_of(OUTPUT_BRANCH_ARG[0]);
        let repo_file_path = matches.value_of(REPO_FILE_ARG);
        let num_commits = matches.value_of(NUM_COMMITS_ARG);
        let num_commits = match num_commits {
            None => None,
            Some(s) => match s.parse::<u32>() {
                Err(_) => None,
                Ok(u) => Some(u),
            },
        };
        Runner {
            repo_file_path: repo_file_path,
            status: 0,
            matches: matches,
            dry_run: is_dry_run,
            verbose: is_verbose,
            should_rebase: is_rebase,
            should_topbase: is_topbase,
            topbase_add_label: false,
            repo_file: RepoFile::new(),
            topbase_top_ref: None,
            repo_original_ref: None,
            current_dir: PathBuf::new(),
            repo: None,
            repo_root_dir: PathBuf::new(),
            include_arg_str: None,
            include_as_arg_str: None,
            exclude_arg_str: None,
            num_commits: num_commits,
            log_p: if is_dry_run { "   # " } else { "" },
            input_branch: None,
            output_branch: if let Some(branch_name) = output_branch {
                Some(branch_name.into())
            } else {
                None
            }
        }
    }

    pub fn add_label_before_topbase(mut self, flag: bool) -> Self {
        self.topbase_add_label = flag;
        self
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
                    die!("Failed to checkout orphan branch");
                }
                // on a new orphan branch our existing files appear in the stage
                // we need to essentially do "git rm -rf ."
                let success = git_helpers::remove_index_and_files(r).is_ok();
                if ! success {
                    die!("Failed to remove git indexed files after making orphan");
                }
            },
            _ => die!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out orphan branch {}", self.log_p, orphan_branch);
        }

        self
    }

    // check the state of the git repository. exit if
    // there are modified files, in the middle of a merge conflict
    // etc...
    pub fn safe_to_proceed(self) -> Self {
        // TODO: also check for other things like:
        // are there files staged? are we resolving a conflict?
        // im just too lazy right now, and this is the most likely scenario
        let args = ["git", "ls-files", "--modified"];
        let output = match exec_helpers::execute(&args) {
            Ok(o) => match o.status {
                0 => o.stdout,
                _ => die!("Failed to run ls-files: {}", o.stderr),
            },
            Err(e) => die!("Failed to run ls-files: {}", e),
        };
        if ! output.is_empty() {
            exit_with_message_and_status(
                "You have modified changes. Please stash or commit your changes before running this command",
                1
            );
        }
        self
    }

    pub fn get_remote_branch_from_args(&self) -> Option<&'a str> {
        let topbase_branch_args = self.matches.occurrences_of(TOPBASE_ARG[0]);
        let rebase_branch_args = self.matches.occurrences_of(REBASE_ARG[0]);
        if topbase_branch_args <= 0 && rebase_branch_args <= 0 {
            return None;
        }

        let use_arg_str = if topbase_branch_args > 0 {
            TOPBASE_ARG[0]
        } else {
            REBASE_ARG[0]
        };

        match &self.matches.value_of(use_arg_str) {
            Some(s) => if *s != "" {
                Some(*s)
            } else {
                None
            },
            None => None,
        }
    }

    pub fn populate_empty_branch_with_remote_commits(self) -> Self {
        let remote_repo = self.repo_file.remote_repo.clone();
        let remote_branch: Option<&str> = match &self.repo_file.remote_branch {
            Some(branch_name) => Some(branch_name.as_str()),
            None => None,
        };
        // if user provided a remote_branch name
        // on the command line, let that override what
        // is present in the repo file:
        let remote_branch = match self.get_remote_branch_from_args() {
            None => remote_branch,
            Some(new_remote_branch) => Some(new_remote_branch),
        };

        match self.repo {
            None => die!("Failed to find repo?"),
            Some(ref r) => {
                match (self.dry_run, &self.input_branch) {
                    (true, Some(branch_name)) => println!("git merge {}", branch_name),
                    (true, None) => println!("git pull {}", remote_repo.unwrap()),
                    (false, Some(branch_name)) => {
                        println!("{}Merging {}", self.log_p, branch_name);
                        git_helpers::merge_branches(&r, &branch_name[..], None);
                    },
                    (false, None) => {
                        let remote_repo_name = remote_repo.clone().unwrap_or("?".into());
                        let remote_branch_name = remote_branch.clone().unwrap_or("".into());
                        let remote_string = if remote_branch_name != "" {
                            format!("{}:{}", remote_repo_name, remote_branch_name)
                        } else { format!("{}", remote_repo_name) };
                        println!("{}Pulling from {}", self.log_p, remote_string);
                        let res = git_helpers3::pull(&remote_repo.unwrap()[..], remote_branch);
                        if res.is_err() {
                            die!("Failed to pull remote repo {}", remote_string);
                        }
                    },
                };
            },
        };
        self
    }

    pub fn rebase(mut self) -> Self {
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
        let err_msg = match exec_helpers::execute(&args) {
            Err(e) => Some(vec![format!("{}", e)]),
            Ok(o) => {
                match o.status {
                    0 => None,
                    _ => Some(vec![o.stderr.lines().next().unwrap().to_string()]),
                }
            },
        };
        if let Some(err) = err_msg {
            self.status = 1;
            let err_details = match self.verbose {
                true => format!("{}", err.join("\n")),
                false => "".into(),
            };
            println!("Failed to rebase\n{}", err_details);
        }
        self
    }

    pub fn get_repo_file(mut self) -> Self {
        // safe to unwrap because its required
        let repo_file_name = self.repo_file_path.unwrap();
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
            Err(_) => die!("Failed to find your current directory. Cannot proceed"),
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
            die!("Failed to change to repository root: {:?}", &self.repo_root_dir);
        }
        if self.verbose {
            println!("{}changed to repository root {}", self.log_p, self.repo_root_dir.display());
        }
        self
    }

    // panic if all dependencies are not met
    pub fn verify_dependencies(self) -> Self {
        if ! exec_helpers::executed_successfully(&["git", "--version"]) {
            die!("Failed to run. Missing dependency 'git'");
        }
        if ! exec_helpers::executed_successfully(&["git", "filter-repo", "--version"]) {
            die!("Failed to run. Missing dependency 'git-filter-repo'");
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
        let err_msg = match exec_helpers::execute(&arg_vec) {
            Ok(o) => match o.status {
                0 => None,
                _ => Some(o.stderr),
            },
            Err(e) => Some(format!("{}", e)),
        };
        if let Some(err) = err_msg {
            die!("Failed to execute: \"{}\"\n{}", arg_vec.join(" "), err);
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
            &include_arg_str,
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
            &include_as_arg_str,
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
            &exclude_arg_str,
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering exclude")
    }
}

pub fn exit_with_message_and_status(msg: &str, status: i32) {
    println!("{}", msg);
    std::process::exit(status);
}

pub fn generate_filter_arg_vec<'a>(
    args: &'a Vec<String>,
    output_branch: &'a str,
) -> Vec<&'a str> {
    let mut arg_vec = vec!["git", "filter-repo"];
    for arg in args {
        arg_vec.push(arg);
    }
    arg_vec.push("--refs");
    arg_vec.push(&output_branch);
    arg_vec.push("--force");

    arg_vec
}

fn get_string_after_last_slash(s: String, slash_type: char) -> String {
    let mut pieces = s.rsplit(slash_type);
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
    GitUrl::parse(remote_repo).is_ok()
}

// try to parse the remote repo
pub fn try_get_repo_name_from_remote_repo(remote_repo: String) -> String {
    let slash_type = MAIN_SEPARATOR;
    let next_slash_type = if slash_type == '/' { '\\' } else { '/' };

    // try to use native slash first:
    let mut repo_name = try_get_repo_name_with_slash_type(&remote_repo, slash_type);
    if repo_name == "" {
        repo_name = try_get_repo_name_with_slash_type(&remote_repo, next_slash_type);
    }

    if repo_name == "" {
        die!("Failed to parse repo_name from remote_repo: {}", remote_repo);
    }

    repo_name
}

pub fn try_get_repo_name_with_slash_type(remote_repo: &String, slash_type: char) -> String {
    let mut out_str = remote_repo.clone().trim_end().to_string();
    if !is_valid_remote_repo(&remote_repo) {
        out_str = "".into();
    }
    if out_str.ends_with(slash_type) {
        out_str.pop();
    }
    if !out_str.contains(slash_type) {
        out_str = "".into();
    }
    out_str = get_string_after_last_slash(out_str, slash_type);
    out_str = get_string_before_first_dot(out_str);

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
                die!("{} is invalid. Must be either a single string, or an even length array of strings", varname);
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

pub fn has_both_topbase_and_rebase(matches: &ArgMatches) -> bool {
    let rebase_args = matches.occurrences_of(REBASE_ARG[0]);
    let topbase_args = matches.occurrences_of(TOPBASE_ARG[0]);
    if rebase_args > 0 && topbase_args > 0 {
        true
    } else {
        false
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn unix_get_repo_name_from_remote_repo_should_try_main_seperator_first() {
        let my_remote_repo = "https://website.com/reponame".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn unix_get_repo_name_from_remote_repo_should_try_main_seperator_first_with_dot() {
        let my_remote_repo = "https://website.com/reponame.git".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn win_get_repo_name_from_remote_repo_should_try_main_seperator_first() {
        let my_remote_repo = "file://some\\path\\reponame".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn unix_get_repo_name_from_remote_repo_should_use_other_path_slash_if_not_found() {
        let my_remote_repo = ".\\Desktop\\reponame".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn win_get_repo_name_from_remote_repo_should_use_other_path_slash_if_not_found() {
        let my_remote_repo = "https://website.com/reponame".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn win_get_repo_name_from_remote_repo_should_use_other_path_slash_if_not_found_with_dot() {
        let my_remote_repo = "https://website.com/reponame.git".into();
        let repo_name = try_get_repo_name_from_remote_repo(my_remote_repo);
        assert_eq!(repo_name, "reponame");
    }

    
    #[test]
    fn validate_git_repo_unix_relative_path() {
        let repo_url = "../test_repo";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: None,
                name: "test_repo".into(),
                owner: None,
                organization: None,
                fullname: "test_repo".into(),
                scheme: git_url_parse::Scheme::File,
                user: None,
                token: None,
                port: None,
                path: "../test_repo".into(),
                git_suffix: false,
                scheme_prefix: false
            }
        )
    }

    #[test]
    fn validate_git_repo_unix_absolute_path_with_file_scheme() {
        let repo_url = "file:///test_repo";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: None,
                name: "test_repo".into(),
                owner: None,
                organization: None,
                fullname: "test_repo".into(),
                scheme: git_url_parse::Scheme::File,
                user: None,
                token: None,
                port: None,
                path: "/test_repo".into(),
                git_suffix: false,
                scheme_prefix: true 
            }
        )
    }

    #[test]
    fn validate_git_repo_win_relative_path() {
        let repo_url = "..\\test_repo";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: None,
                name: "test_repo".into(),
                owner: None,
                organization: None,
                fullname: "test_repo".into(),
                scheme: git_url_parse::Scheme::File,
                user: None,
                token: None,
                port: None,
                path: "../test_repo".into(),
                git_suffix: false,
                scheme_prefix: false 
            }
        )
    }

    #[test]
    fn validate_git_repo_https() {
        let repo_url = "https://website.com/username/reponame";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: Some("website.com".into()),
                name: "reponame".into(),
                owner: Some("username".into()),
                organization: None,
                fullname: "username/reponame".into(),
                scheme: git_url_parse::Scheme::Https,
                user: None,
                token: None,
                port: None,
                path: "/username/reponame".into(),
                git_suffix: false,
                scheme_prefix: true 
            }
        )
    }

    #[test]
    fn validate_git_repo_https_dot_git() {
        let repo_url = "https://website.com/username/reponame.git";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: Some("website.com".into()),
                name: "reponame".into(),
                owner: Some("username".into()),
                organization: None,
                fullname: "username/reponame".into(),
                scheme: git_url_parse::Scheme::Https,
                user: None,
                token: None,
                port: None,
                path: "/username/reponame.git".into(),
                git_suffix: true,
                scheme_prefix: true 
            }
        )
    }

    #[test]
    fn validate_git_repo_ssh_ip_address_with_user_dot_git() {
        let repo_url = "pi@10.10.10.10:/home/reponame.git";
        assert_eq!(
            GitUrl::parse(repo_url).unwrap(),
            GitUrl {
                host: Some("10.10.10.10".into()),
                name: "reponame".into(),
                owner: Some("home".into()),
                organization: None,
                fullname: "home/reponame".into(),
                scheme: git_url_parse::Scheme::Ssh,
                user: Some("pi".into()),
                token: None,
                port: None,
                path: "/home/reponame.git".into(),
                git_suffix: true,
                scheme_prefix: false 
            }
        )
    }
}
