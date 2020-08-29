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

// branch_name should just be the branch name,
// it should not include refs/heads/*
// when this method calls checkout_to_branch, it will format it
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

    let branch_ref = format!("refs/heads/{}", branch_name);
    checkout_to_branch(branch_ref.as_str(), repo)
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

pub fn list_everything_under_tree(
    repo: &Repository,
    tree: git2::Tree,
    indent: &str,
) -> Result<(), git2::Error> {
    for t in tree.iter() {
        println!("{}T:{} ({})", indent, t.id(), t.name().unwrap());
        let t_obj = t.to_object(repo)?;
        match t_obj.kind().unwrap() {
            git2::ObjectType::Blob  => {
                let blob = match t_obj.into_blob() {
                    Ok(b) => b,
                    _ => panic!("failed to turn into blob"),
                };
                println!("{}  B:{}", indent, blob.id());
            }
            git2::ObjectType::Commit => {
            }
            git2::ObjectType::Tree => {
                let t_next = match t_obj.into_tree() {
                    Ok(tn) => tn,
                    _ => panic!("failed to turn into tree"),
                };
                let next_indent = format!("{}  ", indent);
                list_everything_under_tree(repo, t_next, next_indent.as_str())?;
            }
            git2::ObjectType::Any => {
            }
            git2::ObjectType::Tag => {
            }
        };
    }

    Ok(())
}

pub fn list_everything_under_commit(
    repo: &Repository,
    commit: git2::Commit,
) -> Result<(), git2::Error> {
    println!("C:{}", commit.id());
    let tree = commit.tree()?;
    list_everything_under_tree(repo, tree, "  ")?;

    for p in commit.parents() {
        list_everything_under_commit(repo, p)?;
    }

    Ok(())
}

pub fn get_commit_from_ref<'a>(
    repo: &'a Repository,
    refname: &str,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let reference = repo.find_reference(refname)?;
    repo.reference_to_annotated_commit(&reference)
}

pub fn get_plain_commit_from_ref<'a>(
    repo: &'a Repository,
    refname: &str,
) -> Result<git2::Commit<'a>, git2::Error> {
    let reference = repo.find_reference(refname)?;
    match reference.peel_to_commit() {
        Ok(cmt) => Ok(cmt),
        Err(e) => Err(e),
    }
}

pub fn get_commit_from_oid(
    repo: &Repository,
    oid: git2::Oid,
) -> Result<git2::Commit, git2::Error> {
    repo.find_commit(oid)
}

pub fn rebase_each(
    repo: &Repository,
    rebase: &mut git2::Rebase,
) -> Result<(), git2::Error> {
    for _ in 0..rebase.len() {
        let rebase_op_option = rebase.next();
        if let None = rebase_op_option {
            // the next was empty, therefore break out
            // to avoid calling next on a rebase operation that should
            // already be done
            break;
        }
        // we can unwrap safely because the None case
        // was handled above
        let op = rebase_op_option.unwrap();
        match op {
            Ok(rebase_op) => {
                let commit = get_commit_from_oid(&repo, rebase_op.id())?;
                let author = commit.author();
                // here we can rewrite the commit if we want
                // by choosing the commit author as the signature
                // we ensure that the hash stays the same in the
                // fast-forwardable case
                rebase.commit(None, &author, None)?;
            }
            Err(e) => return Err(e),
        };
    }

    Ok(())
}

// example:
// A - B
//  \
//   --C
// if you want to get:
// A - B - C*
// you would do:
// upstream: B, rebase_from C
// note: the branch strings must be refs:
// refs/heads/B and refs/heads/C
pub fn rebase(
    repo: &Repository,
    // the branch that will be used as the "bottom" of the rebase comparison
    upstream_branch: &str,
    // the branch that will be used as the "top" of the rebase comparison
    rebase_from_branch: &str,
    rebase_opts: Option<&mut git2::RebaseOptions<'_>>,
) -> Result<(), git2::Error> {
    let upstream = get_commit_from_ref(&repo, upstream_branch)?;
    let branch = get_commit_from_ref(&repo, rebase_from_branch)?;

    // initialize
    let mut rebase = match repo.rebase(Some(&branch), Some(&upstream), None, rebase_opts) {
        Ok(r) => r,
        Err(e) => {
            return Err(e);
        },
    };

    // do the rebase
    if let Err(e) = rebase_each(repo, &mut rebase) {
        rebase.abort()?;
        return Err(e);
    }

    // finalize
    rebase.finish(None)?;

    Ok(())
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

pub fn fast_forward(
    repo: &Repository,
    target_reference: &mut git2::Reference,
    source_commit: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match target_reference.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(target_reference.name_bytes()).to_string(),
    };
    target_reference.set_target(source_commit.id(), "")?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default().force(),
    ))?;
    Ok(())
}

// given a source commit, merge it into the target branch via the
// branch name. If target branch is None, use current HEAD instead.
pub fn merge<'a>(
    repo: &Repository,
    source_commit: git2::AnnotatedCommit<'a>,
    target_branch: Option<&str>,
) -> Result<(), git2::Error> {
    let analysis = repo.merge_analysis(&[&source_commit])?;
    let refname = match target_branch {
        Some(s) => format!("refs/heads/{}", s),
        None => get_current_branch(repo).unwrap(),
    };

    if analysis.0.is_fast_forward() {
        match repo.find_reference(&refname) {
            Ok(mut r) => { fast_forward(repo, &mut r, &source_commit)?; },
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(&refname, source_commit.id(), true, "")?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        }
    } else {
        panic!("cannot fast-forward. Alternate merge strategies not implements yet");
    }

    Ok(())
}

pub fn merge_branches(
    repo: &Repository,
    source_branch: &str,
    target_branch: Option<&str>,
) -> Result<(), git2::Error> {
    let refname = format!("refs/heads/{}", source_branch);
    let source_ref = repo.find_reference(&refname[..])?;
    let source_commit = repo.reference_to_annotated_commit(&source_ref)?;
    merge(repo, source_commit, target_branch)
}

pub fn pull(
    repo: &Repository,
    remote_name: &str,
    remote_branch_name: Option<&str>,
) -> Result<(), git2::Error> {
    let mut remote = repo.remote_anonymous(remote_name)?;
    let remote_branch = remote_branch_name.unwrap_or("master");
    let fetched_commit = fetch(
        repo,
        &[remote_branch],
        &mut remote,
    ).unwrap();
    merge(repo, fetched_commit, None)
}

pub fn fetch<'a>(
    repo: &'a Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    remote.fetch(
        refs,
        Some(&mut git2::FetchOptions::new()
            .download_tags(git2::AutotagOption::All)
        ),
        None
    )?;

    let fetched_commit = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetched_commit)?)
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
