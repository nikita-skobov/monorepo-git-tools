use clap::ArgMatches;
use std::collections::HashSet;

use super::git_helpers;
use super::exec_helpers;
use super::split::Runner;

pub trait Topbase {
    fn topbase(self) -> Self;
}

impl<'a> Topbase for Runner<'a> {
    fn topbase(self) -> Self {
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
            // if current branch only has one commit, dont use the <branch>~1
            // git rebase syntax. it will cause git rebase to fail
            let rebase_last_one = if num_commits_of_current > 1 {
                "~1"
            } else {
                ""
            };
            let last_commit_arg = format!("{}{}", current_branch, rebase_last_one);
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

pub fn run_topbase(matches: &ArgMatches) {

}
