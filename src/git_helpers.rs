use git2::Repository;
use git2::Error;
use std::path::PathBuf;

pub fn get_repository_and_root_directory(dir: &PathBuf) -> (Repository, PathBuf) {
    let repo = match Repository::discover(dir) {
        Err(e) => panic!("Failed to find or open repository from {} - {}", dir.display(), e),
        Ok(repo) => repo,
    };

    let mut pathbuf = repo.path().to_path_buf();
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


#[cfg(test)]
mod test {
    use super::*;

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
