// use super::interact;
use super::cli::MgtCommandSync;
use super::core;
use super::die;
use super::repo_file;
use std::{io, path::PathBuf};
use crate::{split_out::validate_repo_file_res, ioerr};

pub struct RepoSplitItem {
    pub repofilepath: PathBuf,
    pub split_branch: String,
    pub topbase_branch: String,
}

/// generate a 'random' string that will hopefully not clash with any
/// existing branch name
pub fn get_repo_split_branch_name(split_or_topbase: &str, index: usize) -> String {
    let random_str = "ecktzpkyjon";
    format!("mgt-{}-{}-{}", random_str, split_or_topbase, index)
}

pub fn get_all_repo_files_ex(list: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out_vec = vec![];
    for path in list {
        if path.is_dir() {
            let all_paths_in_dir = match core::get_all_repo_files(path, true, false) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Skipping {:?} because failed to read repo files:\n{}\n", path, e);
                    continue;
                }
            };

            for p in all_paths_in_dir {
                out_vec.push(PathBuf::from(p));
            }
        } else {
            out_vec.push(path.clone());
        }
    }
    out_vec
}

pub fn sync_repo_file(
    repo_split_item: &RepoSplitItem,
    cmd: &MgtCommandSync
) -> io::Result<()> {
    let mut repo_file = repo_file::parse_repo_file_from_toml_path_res(&repo_split_item.repofilepath)?;
    // the output branch doesnt matter here because
    // we will use our own output branch anyway
    let mut out_branch = None;
    validate_repo_file_res(&mut repo_file, &mut out_branch)?;

    let mut out_branch = Some(repo_split_item.split_branch.clone());
    core::make_and_checkout_output_branch_res(&mut out_branch, false, false)?;


    Ok(())
}

pub fn canonicalize_all_repo_file_paths(paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out_paths = vec![];
    for p in paths {
        match p.canonicalize() {
            Ok(canon) => {
                out_paths.push(canon);
            }
            Err(e) => {
                eprintln!("Not including {:?} because failed to canonicalize path\n{}", p, e);
            }
        };
    }
    out_paths
}

pub fn attempt_cleanup(delete_branches: Vec<String>, starting_branch: String) -> io::Result<()> {

    Ok(())
}

pub fn attempt_cleanup_or_die(delete_branches: Vec<String>, starting_branch: String) {
    if let Err(e) = attempt_cleanup(delete_branches, starting_branch) {
        eprintln!("Error while attempting to cleanup sync command:\n{}", e);
        std::process::exit(1);
    }
}

// TODO: what about syncing changes from remote to local?
// right now its easiest to implement syncing local code to remote
// because topbase can do that for us, but think about
// how we can sync remote to local?
// would we do:
// foreach repofile in repofiles:
//    - sync local to remote
//    - sync remote to local (many sync commits)
// OR:
// foreach repofile in repofiles:
//    - sync local to remote
// foreach repofile in repofiles:
//    - aggregate changes
// - sync local to remote (single sync commit)

pub fn run_sync(cmd: &mut MgtCommandSync) {
    // before we go to the repo root, we want to canonicalize
    // all of the paths the user provided, otherwise they wont work anymore
    // from a new directory
    cmd.repo_files = canonicalize_all_repo_file_paths(&cmd.repo_files);
    core::verify_dependencies();
    core::go_to_repo_root();
    core::safe_to_proceed();

    let starting_branch_name = core::get_current_ref().unwrap_or_else(|| {
        die!("Failed to get current branch name. Cannot continue")
    });
    let mut all_repo_files = get_all_repo_files_ex(&cmd.repo_files);
    println!("Found {:#?} repo files to sync", all_repo_files);
    println!("Found {} repo files to sync", all_repo_files.len());

    let mut branches_to_delete = vec![];
    for (index, repo_file) in all_repo_files.drain(..).enumerate() {
        let potential_err = format!("Error trying to sync {:?} :", repo_file);
        let repo_split_item = RepoSplitItem {
            repofilepath: repo_file,
            split_branch: get_repo_split_branch_name("split", index),
            topbase_branch: get_repo_split_branch_name("topbase", index),
        };
        branches_to_delete.push(repo_split_item.split_branch.clone());
        branches_to_delete.push(repo_split_item.topbase_branch.clone());

        // TODO: need a --no-cleanup flag
        // to avoid trying to cleanup state if theres a failure
        if let Err(e) = sync_repo_file(&repo_split_item, cmd) {
            eprintln!("{}\n{}", potential_err, e);
            if cmd.fail_fast {
                attempt_cleanup_or_die(branches_to_delete, starting_branch_name);
                std::process::exit(1);
            }
        }
    }

    attempt_cleanup_or_die(branches_to_delete, starting_branch_name);
}
