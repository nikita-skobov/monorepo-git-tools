use git2::Repository;
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


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_number_of_commits_works() {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(".");
        let (repo, _) = get_repository_and_root_directory(&pathbuf);
        let num_commits = get_number_of_commits_in_branch("master", &repo);
        let num = num_commits.unwrap_or(0);
        println!("num: {}", num);
        assert!(num >= 10 && num <= 99999);
    }
}
