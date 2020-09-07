// this file defines the base behavior or splitting
// and running a split-X command
use std::env;
use std::path::PathBuf;
use std::collections::HashSet;
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

    pub fn topbase(self) -> Self {
        let repo = match self.repo {
            Some(ref r) => r,
            None => panic!("failed to get repo?"),
        };

        let current_branch = match git_helpers::get_current_ref(repo) {
            Some(s) => s,
            None => {
                println!("Failed to get current branch. not going to rebase");
                return self;
            },
        };

        let upstream_branch = match self.repo_original_ref {
            Some(ref branch) => branch.clone(),
            None => {
                println!("Failed to get repo original ref. Not going to rebase");
                return self;
            },
        };

        let all_upstream_blobs = get_all_blobs_in_branch(upstream_branch.as_str());
        let all_commits_of_current = match git_helpers::get_all_commits_from_ref(repo, current_branch.as_str()) {
            Ok(v) => v,
            Err(e) => panic!("Failed to get all commits! {}", e),
        };

        let num_commits_of_current = all_commits_of_current.len();
        let mut num_commits_to_take = 0;
        let mut rebase_data = vec![];
        // for every commit in the current branch (the branch going to be rebased)
        // check if every single blob of every commit exists in the upstream branch.
        // as soon as we a commit of this current branch that has all of its blobs
        // exists in upstream, then we break, and run out interactive rebase that we
        // are building
        for c in all_commits_of_current {
            // I think we want to skip merge commits, because thats what git rebase
            // interactive does by default. also, is it safe to assume
            // any commit with > 1 parent is a merge commit?
            if c.parent_count() > 1 {
                continue;
            }

            let mut current_commit_blobs = HashSet::new();
            get_all_blobs_from_commit(&c.id().to_string()[..], &mut current_commit_blobs);
            let mut all_blobs_exist = true;
            for b in current_commit_blobs {
                if ! all_upstream_blobs.contains(&b) {
                    all_blobs_exist = false;
                    break;
                }
            }
            if all_blobs_exist {
                break;
            }
            num_commits_to_take += 1;
            let rebase_interactive_entry = format!("pick {} {}\n", c.id(), c.summary().unwrap());
            rebase_data.push(rebase_interactive_entry);
        }

        // need to reverse it because git rebase interactive
        // takes commits in order of oldest to newest, but
        // we parsed them from newest to oldest
        rebase_data.reverse();

        // we just want to use the actual branch names, not the ref name
        let current_branch = current_branch.replace("refs/heads/", "");
        let upstream_branch = upstream_branch.replace("refs/heads/", "");

        // log the special cases
        if num_commits_to_take == 0 {
            // if we have found that the most recent commit of current_branch already exists
            // on the upstream branch, we should just rebase normally (so that the branch can be fast-forwardable)
            // instead of rebasing interactively
            println!("{}most recent commit of {} exists in {}. rebasing non-interactively", self.log_p, current_branch, upstream_branch);
        } else if num_commits_to_take == num_commits_of_current {
            // if we are trying to topbase on a branch that hasnt been rebased yet,
            // we dont need to topbase, and instead we need to do a regular rebase
            println!("{}no commit of {} exists in {}. rebasing non-interactively", self.log_p, current_branch, upstream_branch);
        }

        // if there's nothing to topbase, then we want to just
        // rebase the last commit onto the upstream branch.
        // this will allow our current branch to be fast-forwardable
        // onto upstream (well really its going to be the exact same branch)
        if num_commits_to_take == 0 {
            let last_commit_arg = format!("{}~1", current_branch);
            let args = [
                "git", "rebase", "--onto",
                upstream_branch.as_str(),
                last_commit_arg.as_str(),
                current_branch.as_str()
            ];

            if self.dry_run {
                let arg_str = args.join(" ");
                println!("{}", arg_str);
                return self;
            }

            match exec_helpers::execute(&args) {
                Err(e) => panic!("Failed to rebase: {}", e),
                Ok(_) => (),
            };
            return self;
        }

        // if we need to topbase the entirety of the current branch
        // it will be better to do a regular rebase
        if num_commits_to_take == num_commits_of_current {
            if self.dry_run {
                println!("git rebase {}", upstream_branch);
                return self;
            }

            let args = [
                "git", "rebase", upstream_branch.as_str(),
            ];
            match exec_helpers::execute(&args) {
                Err(e) => panic!("Failed to rebase: {}", e),
                Ok(_) => (),
            };
            return self;
        }

        if self.dry_run || self.verbose {
            // since we are already on the rebase_from_branch
            // we dont need to specify that in the git command
            // the below command implies: apply rebased changes in
            // the branch we are already on
            println!("rebase_data=\"{}\"", rebase_data.join(""));
            println!("GIT_SEQUENCE_EDITOR=\"echo $rebase_data >\" git rebase -i --onto {} {}~{} {}",
                upstream_branch,
                current_branch,
                num_commits_to_take,
                current_branch,
            );
            if self.dry_run {
                return self;
            }
        }

        // rebase_data="pick <hash> <msg>
        // pick <hash> <msg>
        // pick <hash> <msg>
        // "
        // rebase_command="echo \"$rebase_data\""
        // GIT_SEQUENCE_EDITOR="$rebase_command >" git rebase -i --onto bottom top~3 top
        let upstream_arg = format!("{}~{}", current_branch, num_commits_to_take);
        let args = [
            "git", "rebase", "-i",
            "--onto", upstream_branch.as_str(),
            upstream_arg.as_str(),
            current_branch.as_str(),
        ];
        let rebase_data_str = rebase_data.join("");
        let rebase_data_str = format!("echo \"{}\" >", rebase_data_str);

        match exec_helpers::execute_with_env(
            &args,
            &["GIT_SEQUENCE_EDITOR"],
            &[rebase_data_str.as_str()],
        ) {
            Err(e) => println!("Failed to rebase: {}", e),
            Ok(o) => {
                if o.status != 0 {
                    println!("Failed to rebase: {} {}", o.stdout, o.stderr);
                }
            },
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

// run a git diff-tree on the commit id, and parse the output
// and insert every blob id into the provided blob hash set
pub fn get_all_blobs_from_commit(
    commit_id: &str,
    blob_set: &mut HashSet<String>,
) {
    // the diff filter is VERY important... A (added), M (modified), C (copied)
    // theres a few more like D (deleted), but I don't think we want the D because
    // *I think* we only care about blobs that EXIST at a given point in time...
    // maybe this might change later
    let args = [
        "git", "diff-tree", commit_id, "-r", "--root",
        "--diff-filter=AMC", "--pretty=oneline"
    ];
    match exec_helpers::execute(&args) {
        Err(e) => panic!("Failed to get blobs from commit {} : {}", commit_id, e),
        Ok(out) => {
            if out.status != 0 { panic!("Failed to get blobs from commit {} : {}", commit_id, out.stderr); }
            for l in out.stdout.lines() {
                // lines starting with colons are the lines
                // that contain blob ids
                if ! l.starts_with(':') { continue; }
                let items = l.split_whitespace().collect::<Vec<&str>>();
                // there are technically 6 items from this output:
                // the last item (items[5]) is a path to the file that this blob
                // is for (and the array could have more than 6 if file names
                // have spaces in them). But we only care about the first 5:
                let (
                    mode_prev, mode_next,
                    blob_prev, blob_next,
                    diff_type
                ) = (items[0], items[1], items[2], items[3], items[4]);
                // now blob_prev will be all zeros if diff_type is A
                // however, for other diff_types, it will be a valid blob.
                // it is my assumption that we don't need it because
                // it probably exists in one of the other commits as a blob_next.
                // maybe this will need to change, but for now, I think it is
                // sufficient to just get the blob_next
                blob_set.insert(blob_next.into());
            }
        }
    };
}

// perform a rev-list of the branch name to get a list of all commits
// then get every single blob from every single commit, and return
// a hash set containing unique blob ids
pub fn get_all_blobs_in_branch(branch_name: &str) -> HashSet<String> {
    // first get all commits from this branch:
    let args = [
        "git", "rev-list", branch_name,
    ];

    // need the stdout to live outside the match so that the vec of strings
    // lives outside the match
    let mut out_stdout = "".into();
    let commit_ids = match exec_helpers::execute(&args) {
        Err(e) => panic!("Failed to get all blobs of {} : {}", branch_name, e),
        Ok(out) => {
            if out.status != 0 { panic!("Failed to get all blobs of {} : {}", branch_name, out.stderr); }
            out_stdout = out.stdout;
            out_stdout.split_whitespace().collect::<Vec<&str>>()
        },
    };

    let mut blob_set = HashSet::new();
    for commit_id in commit_ids.iter() {
        get_all_blobs_from_commit(commit_id, &mut blob_set);
    }

    return blob_set;
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
