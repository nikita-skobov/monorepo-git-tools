use git2::Repository;
use git2::Error;
use std::path::PathBuf;
use std::path::Path;
use std::fs;
use std::str::from_utf8;

fn remove_git_from_path_buf(pathbuf: &mut PathBuf) -> PathBuf {
    match &pathbuf.file_name() {
        Some(p) => {
            match p.to_str() {
                Some(s) => {
                    if s == ".git" {
                        pathbuf.pop();
                    }
                },
                _ => (),
            }
        },
        _ => (),
    };
    return pathbuf.clone();
}

pub fn get_repository_and_root_directory(dir: &PathBuf) -> (Repository, PathBuf) {
    let repo = match Repository::discover(dir) {
        Err(e) => panic!("Failed to find or open repository from {} - {}", dir.display(), e),
        Ok(repo) => repo,
    };

    let pathbuf = remove_git_from_path_buf(
        &mut repo.path().to_path_buf()
    );

    return (repo, pathbuf);
}


pub fn get_number_of_commits_in_branch(
    branch_name: &str, repo: &Repository
) -> Result<i32, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::NONE)?;
    let latest_commit_id = repo.revparse_single(branch_name)?.id();
    revwalk.push(latest_commit_id)?;

    let mut num_commits = 0;
    for _ in revwalk {
        num_commits += 1;
    }

    return Ok(num_commits);
}

pub fn branch_exists(
    branch_name: &str,
    repo: &Repository,
) -> bool {
    let local_branch = git2::BranchType::Local;
    match repo.find_branch(branch_name, local_branch) {
        Ok(_) => true,
        _ => false,
    }
}

pub fn get_current_branch(
    repo: &Repository
) -> Result<String, git2::Error> {
    let reference = match repo.head() {
        Ok(refrnc) => refrnc,
        Err(e) => {
            let msg: String = e.message().into();
            if ! msg.contains("' not found") {
                // some message we dont care about
                // so just return the error as usual
                return Err(e);
            }
            // if it says "ref 'refs/something' not found"
            // then it exists, its just not a valid head,
            // so we want to return that anyway
            return Ok(msg.split("'").skip(1).take(1).collect::<Vec<&str>>()[0].into());
        },
    };

    match reference.name() {
        Some(name) => Ok(name.to_string()),
        None => {
            let code = git2::ErrorCode::GenericError;
            let class = git2::ErrorClass::None;
            let message = format!("Cannot get current HEAD reference name. It is probably a malformed UTF-8 issue");

            return Err(Error::new(code, class, message));
        }
    }
}

pub fn make_orphan_branch_and_checkout(
    branch_name: &str,
    repo: &Repository,
) -> Result<(), git2::Error> {
    if branch_exists(branch_name, repo) {
        let code = git2::ErrorCode::Exists;
        let class = git2::ErrorClass::Checkout;
        let message = format!("Cannot checkout to orphan branch: {}. It already exists", branch_name);

        return Err(Error::new(code, class, message));
    }

    checkout_to_branch(branch_name, repo)
}

pub fn checkout_to_branch(
    branch_name: &str,
    repo: &Repository,
) -> Result<(), git2::Error> {
    repo.set_head(branch_name)
}

pub fn get_head_commit(
    repo: &Repository,
) -> Result<git2::Commit, git2::Error> {
    let current_ref = get_current_branch(repo)?;
    let current_oid = repo.refname_to_id(current_ref.as_str())?;
    repo.find_commit(current_oid)
}

pub fn make_new_branch_from_head(
    repo: &Repository,
    branch_name: &str,
) -> Result<(), git2::Error> {
    let current_head_commit = get_head_commit(repo)?;
    let branch = repo.branch(branch_name, &current_head_commit, false)?;
    Ok(())
}

// basically git rm -rf .
// it gets all paths from the index
// and then removes all of them one by one
pub fn remove_index_and_files(
    repo: &Repository
) -> Result<(), git2::Error> {
    let mut index = repo.index().expect("FAILED TO GET INDEX");
    let mut files_to_delete: Vec<PathBuf> = vec![];
    for entry in index.iter() {
        let p = from_utf8(&entry.path).unwrap();
        files_to_delete.push(p.into());
    }
    // we probably want to write index before
    // deleting the files, because if the index change fails
    // we dont want to delete the files
    index.clear()?;
    index.write()?;

    // we only check if we have successfully removed the first file
    // otherwise whats the point of erroring if we remove one or more files
    // but fail on another one?
    let mut file_removed = false;
    for f in &files_to_delete {
        if ! file_removed {
            let result = fs::remove_file(f);
            if result.is_err() {
                panic!("Failed to remove file {}. Stopping operation without modifying index", f.display());
            }
            file_removed = true;
        } else {
            fs::remove_file(f);
        }
    }

    // we need to do this to delete empty directories that were left over
    // from deleting the above files... yeah kinda slow but idk a better way
    for f in &files_to_delete {
        let mut parent = f.parent();
        while parent.is_some() {
            let parent_path = parent.unwrap();
            if parent_path.is_dir() {
                // we dont care if this errors.
                // an error will occur if the directory is not empty, which
                // is fine because if its not empty we dont want to delete it anyway
                fs::remove_dir(parent_path);
            }
            parent = parent_path.parent();
        }
    }

    Ok(())
}

pub fn make_new_branch_from_head_and_checkout(
    repo: &Repository,
    branch_name: &str
) -> Result<(), git2::Error> {
    make_new_branch_from_head(repo, branch_name)?;
    let branch_ref = format!("refs/heads/{}", branch_name);
    checkout_to_branch(branch_ref.as_str(), repo)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // // TODO: write this test
    // // ensure branch exists,
    // // and points to same commit as master. idk how to do that yet
    // fn make_new_branch_works() {
    //     let mut pathbuf = PathBuf::new();
    //     pathbuf.push(".");
    //     let (repo, _) = get_repository_and_root_directory(&pathbuf);
    //     make_new_branch_from_head_and_checkout(&repo, "testbranch");
    // }

    #[test]
    #[cfg_attr(not(feature = "gittests"), ignore)]
    fn get_number_of_commits_works() {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(".");
        let (repo, _) = get_repository_and_root_directory(&pathbuf);
        let num_commits = get_number_of_commits_in_branch("master", &repo);
        let num = num_commits.unwrap_or(0);
        println!("num: {}", num);
        assert!(num >= 10 && num <= 99999);
    }

    #[test]
    #[cfg_attr(not(feature = "gittests"), ignore)]
    fn make_orphan_branch_and_checkout_works() {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(".");
        let (repo, _) = get_repository_and_root_directory(&pathbuf);
        let testbranchname = "refs/heads/blahbranchblaaaah";
        // make sure the branch name doesnt exist yet
        assert_eq!(branch_exists(testbranchname, &repo), false);
        let res = make_orphan_branch_and_checkout(testbranchname, &repo);
        assert_eq!(res.is_ok(), true);
        // current head should point to blahbranch:
        assert_eq!(get_current_branch(&repo).unwrap(), testbranchname.to_string());
        // checkout back to master because thats what the other tests depend on
        checkout_to_branch("refs/heads/master", &repo);
    }

    #[test]
    #[cfg_attr(not(feature = "gittests"), ignore)]
    fn get_current_branch_works() {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(".");
        let (repo, _) = get_repository_and_root_directory(&pathbuf);
        assert_eq!(get_current_branch(&repo).unwrap(), "refs/heads/master".to_string());
    }
}
