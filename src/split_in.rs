use clap::ArgMatches;

use super::commands::INPUT_BRANCH_ARG;
use super::commands::AS_SUBDIR_ARG;
use super::commands::REPO_URI_ARG;
use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::git_helpers;
use super::exec_helpers;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;
use std::convert::From;
use std::fs;
use std::path::Path;
use std::fmt::Display;
use std::{collections::HashSet, path::PathBuf};

pub trait SplitIn {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
    fn make_and_checkout_output_branch(self) -> Self;
}

impl<'a> SplitIn for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        self.input_branch = match self.matches.value_of(INPUT_BRANCH_ARG) {
            None => None,
            Some(branch_name) => {
                match &self.repo {
                    None => panic!("Failed to find repo for some reason"),
                    Some(ref repo) => {
                        if ! git_helpers::branch_exists(branch_name, repo) {
                            panic!("You specified an input branch of {}, but that branch was not found", branch_name);
                        }
                        Some(branch_name.into())
                    },
                }
            },
        };

        let missing_output_branch = self.output_branch.is_none();
        let missing_input_branch = self.input_branch.is_none();
        let missing_repo_name = self.repo_file.repo_name.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();

        if missing_remote_repo && missing_input_branch && ! missing_output_branch {
            panic!("Must provide either repo_name in your repofile, or specify a --{} argument", INPUT_BRANCH_ARG);
        }

        if missing_include && missing_include_as {
            panic!("Must provide either include or include_as in your repofile");
        }

        if missing_repo_name && !missing_remote_repo && missing_output_branch {
            let output_branch_str = try_get_repo_name_from_remote_repo(
                self.repo_file.remote_repo.clone().unwrap()
            );
            self.repo_file.repo_name = Some(output_branch_str.clone());
            self.output_branch = Some(output_branch_str);
        } else if missing_output_branch && ! missing_repo_name {
            // make the repo_name the output branch name
            self.output_branch = Some(self.repo_file.repo_name.clone().unwrap());
        } else if missing_output_branch && ! missing_input_branch {
            // make the output_branch the name of the input_branch -reverse
            let output_branch_str = format!("{}-reverse", self.input_branch.clone().unwrap());
            self.output_branch = Some(output_branch_str);
        }

        panic_if_array_invalid(&self.repo_file.include, true, "include");
        panic_if_array_invalid(&self.repo_file.include_as, false, "include_as");

        self
    }

    fn generate_arg_strings(mut self) -> Self {
        let include_as_arg_str = generate_split_out_arg_include_as(&self.repo_file);
        let exclude_arg_str = generate_split_out_arg_exclude(&self.repo_file);
        let include_arg_str = generate_split_out_arg_include(&self.repo_file);

        if self.verbose {
            println!("{}include_arg_str: {}", self.log_p, include_arg_str.join(" "));
            println!("{}include_as_arg_str: {}", self.log_p, include_as_arg_str.join(" "));
            println!("{}exclude_arg_str: {}", self.log_p, exclude_arg_str.join(" "));
        }

        if include_arg_str.len() != 0 {
            self.include_arg_str = Some(include_arg_str);
        }
        if include_as_arg_str.len() != 0 {
            self.include_as_arg_str = Some(include_as_arg_str);
        }
        if exclude_arg_str.len() != 0 {
            self.exclude_arg_str = Some(exclude_arg_str);
        }

        self
    }

    fn make_and_checkout_output_branch(mut self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();

        self.make_and_checkout_orphan_branch(output_branch_name.as_str())
    }
}

pub fn path_is_ignored(path: &PathBuf) -> bool {
    let path_str = format!("{:?}", path);
    if path.starts_with(".git") || path.starts_with("./.git") {
        return true;
    }

    exec_helpers::executed_successfully(
        &["git", "check-ignore", path.to_str().unwrap()])
}

/// get a vector of paths of files that git knows about
/// starting from the root of the repo
pub fn get_commited_paths() -> Vec<PathBuf> {
    let data = exec_helpers::execute(&["git", "ls-files"]);
    let data = match data {
        Err(e) => panic!("Failed to list git committed files: {}", e),
        Ok(d) => {
            if d.status != 0 {
                panic!("Failed to list git committed files: {}", d.stderr);
            }
            d.stdout
        },
    };

    let mut out_vec: Vec<PathBuf> = vec![];
    for f in data.split('\n').into_iter() {
        let pathbuf_with_file = PathBuf::from(f);
        let mut pathbuf_without_file = pathbuf_with_file.clone();
        pathbuf_without_file.pop();
        out_vec.push(pathbuf_with_file);
        out_vec.push(pathbuf_without_file);
    }
    out_vec
}

// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
// include gets taken as is, but for include_as, we only care about
// what the destination is because we will run
// the include filter after we rename, so we want the renamed versions
pub fn generate_split_out_arg_include(repofile: &RepoFile) -> Vec<String> {
    let include = if let Some(include) = &repofile.include {
        include.clone()
    } else {
        vec![]
    };
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        vec![]
    };

    let mut out_vec = vec![];
    for path in include {
        out_vec.push("--path".to_string());
        if path == " " {
            out_vec.push("".into());
        } else {
            out_vec.push(path);
        }
    }
    // include_as is more difficult because the indices matter
    // for splitting out, the even indices are the local
    // paths, so those are the ones we want to include
    for path in include_as.iter().step_by(2) {
        out_vec.push("--path".to_string());
        if path == " " {
            out_vec.push("".into())
        } else {
            out_vec.push(path.into());
        }
    }

    out_vec
}


// iterate over the include_as variable, and generate a
// string of args that can be passed to git-filter-repo
pub fn generate_split_out_arg_include_as(
    repofile: &RepoFile,
) -> Vec<String> {
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        return vec![];
    };

    let valid_git_files = get_commited_paths();

    generate_split_arg_include_as(&include_as, valid_git_files)
}

// given a vec of include_as pairs,
// iterate over the odd-indexed elements.
// if the path is a file include it as is
// if the path is a folder, iterate over the files in that folder
// recursively, and add a --path-rename file:file instead of folder:folder
pub fn generate_split_arg_include_as<T: AsRef<str> + AsRef<Path> + Display>(
    include_as: &[T],
    valid_files: Vec<PathBuf>
) -> Vec<String> {
    let sources = include_as.iter().skip(1).step_by(2);
    let destinations = include_as.iter().skip(0).step_by(2);

    let pairs = sources.zip(destinations);

    let mut unique_files = HashSet::new();
    let mut out_vec = vec![];
    for (src, dest) in pairs {
        let src_str: &str = src.as_ref();
        let src_pb = if src_str == " " {
            PathBuf::from(".")
        } else {
            PathBuf::from(src_str)
        };

        match src_pb.is_file() {
            // files can keep their existing mapping
            true => {
                unique_files.insert(src.to_string());
                out_vec.push("--path-rename".into());
                out_vec.push(format!("{}:{}", src, dest));
            },
            // but for folders, we should iterate recursively into the
            // folder and add every file mapping explicitly
            false => {
                let paths = gen_include_as_arg_files_from_folder(
                    dest.as_ref(),
                    src_pb,
                    &valid_files,
                    &mut unique_files
                );
                for p in paths {
                    out_vec.push(p);
                }
            },
        }
    }

    out_vec
}

pub fn get_files_recursively<F: Copy>(
    dir: PathBuf, file_vec: &mut Vec<PathBuf>, should_ignore: F
) -> std::io::Result<()>
    where F: Fn(&PathBuf) -> bool
{
    for entry in fs::read_dir(dir)? {
        let dir_entry = entry?;
        let dir_path = dir_entry.path();
        if should_ignore(&dir_path.to_path_buf()) {
            continue;
        }

        if dir_path.is_file() {
            file_vec.push(dir_path.to_path_buf());
        } else {
            get_files_recursively(dir_path.to_path_buf(), file_vec, should_ignore)?;
        }
    }
    Ok(())
}

pub fn gen_include_as_arg_files_from_folder(
    dest: &str,
    src: PathBuf,
    valid_files: &Vec<PathBuf>,
    unique_files: &mut HashSet<String>,
) -> Vec<String> {
    let mut file_vec = vec![];
    // ignore any file that isn't in the list of valid files
    let should_ignore = |p: &PathBuf| {
        let mut use_p = p.clone();
        if p.starts_with("./") {
            use_p = use_p.strip_prefix("./").unwrap().to_path_buf();
        }
        !valid_files.contains(&use_p)
    };

    let all_files_recursively = get_files_recursively(src.clone(), &mut file_vec, should_ignore);
    if all_files_recursively.is_err() {
        panic!("Error reading dir recursively: {:?}", src);
    }

    let mut out_vec = vec![];
    for f in file_vec.iter() {
        let src_is_dot = src.to_str().unwrap() == ".";
        let src_str = if ! src_is_dot {
            &src.to_str().unwrap()
        } else {
            ""
        };
        // we will replace the original src prefix
        // with the provided dest prefix, so strip the current prefix here
        let f_str = f.strip_prefix(&src).unwrap().to_str().unwrap();

        // the replace here will prevent properly including
        // any files that have a backslash in them. I think
        // it would be too difficult to support such files for now.
        // also this replacement will still work on windows because
        // git-filter-repo actually expects the paths to have
        // unix-like path separators
        let new_src = format!("{}{}", src_str, f_str);
        let new_src = new_src.replace("\\", "/");
        let new_dest = format!("{}{}", dest, f_str);
        let new_dest = new_dest.replace("\\", "/");
        // we only want to add unique paths, so
        // if we already added this one, dont add it again
        if unique_files.contains(&new_src) {
            continue;
        }
        unique_files.insert(new_src.clone());

        out_vec.push("--path-rename".into());
        out_vec.push(format!("{}:{}", new_src, new_dest));
    }

    out_vec
}

pub fn generate_split_out_arg_exclude(repofile: &RepoFile) -> Vec<String> {
    let mut out_vec = vec![];
    match &repofile.exclude {
        None => return out_vec,
        Some(v) => {
            out_vec.push("--invert-paths".to_string());
            for path in v {
                out_vec.push("--path".to_string());
                if path == " " {
                    out_vec.push("".into());
                } else {
                    out_vec.push(path.clone());
                }
            }
            out_vec
        },
    }
}

pub fn run_split_in(matches: &ArgMatches) {
    let runner = Runner::new(matches)
        .get_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .save_current_ref()
        .verify_dependencies()
        .validate_repo_file()
        .change_to_repo_root()
        .make_and_checkout_output_branch()
        .populate_empty_branch_with_remote_commits()
        .generate_arg_strings();

    let log_p = runner.log_p.clone();
    let temp_branch = runner.output_branch.clone().unwrap_or("\"\"".into());
    println!("{}Running filter commands on temporary branch: {}", log_p, temp_branch);
    let mut runner = runner
        .filter_exclude()
        .filter_include_as()
        .filter_include();

    // if we should rebase (or topbase), we need to refresh the repository index
    // since the above filtering commands changed some stuff that
    // our in-memory repository representation does not know about
    // idk if this is the best way to do it, but its simplest
    if runner.should_topbase {
        println!("{}Topbasing", log_p);
        use super::topbase::Topbase;
        runner = runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        println!("{}Rebasing", log_p);
        runner = runner.get_repository_from_current_dir().rebase();
    }

    match runner.status {
        0 => println!("{}Success!", log_p),
        c => {
            std::process::exit(c);
        },
    };
}

pub fn run_split_in_as(matches: &ArgMatches) {
    // should be safe to unwrap because its a required argument
    let include_as_src = matches.value_of(AS_SUBDIR_ARG).unwrap();
    let repo_uri = matches.value_of(REPO_URI_ARG).unwrap();
    let mut runner = Runner::new(matches);
    runner.repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    runner.repo_file.remote_repo = Some(repo_uri.into());

    let runner = runner.save_current_dir()
        .get_repository_from_current_dir()
        .save_current_ref()
        .verify_dependencies()
        .validate_repo_file()
        .change_to_repo_root()
        .make_and_checkout_output_branch()
        .populate_empty_branch_with_remote_commits()
        .generate_arg_strings();

    let log_p = runner.log_p.clone();
    let temp_branch = runner.output_branch.clone().unwrap_or("\"\"".into());
    println!("{}Running filter commands on temporary branch: {}", log_p, temp_branch);
    let mut runner = runner
        .filter_exclude()
        .filter_include_as()
        .filter_include();

    if runner.should_topbase {
        println!("{}Topbasing", log_p);
        use super::topbase::Topbase;
        runner = runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        println!("{}Rebasing", log_p);
        runner = runner.get_repository_from_current_dir().rebase();
    }

    match runner.status {
        0 => println!("{}Success!", log_p),
        c => {
            std::process::exit(c);
        },
    };
}


#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    // this test assumes theres a target/ directory with rust builds
    // this is gitignored, so shouldnt show up on git ls-files
    #[test]
    fn get_commited_paths_should_not_contain_ignored_files() {
        let files = get_commited_paths();
        assert!(files.contains(&PathBuf::from("Cargo.toml")));
        assert!(files.contains(&PathBuf::from("src/split_in.rs")));
        assert!(!files.contains(&PathBuf::from("target/release/mgt")));
    }
    #[test]
    fn get_commited_paths_should_contain_the_folder_leading_to_the_file() {
        let files = get_commited_paths();
        assert!(files.contains(&PathBuf::from("test/")));
        assert!(files.contains(&PathBuf::from("test/general/")));
        assert!(files.contains(&PathBuf::from("test/general/end-to-end.bats")));
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_return_the_mapping_as_is_if_src_is_file() {
        let include_as = vec![
            "src/lib/Cargo.toml", "Cargo.toml",
        ];
        let pathbufs = vec![
            PathBuf::from("Cargo.toml")
        ];
        let s = generate_split_arg_include_as(&include_as, pathbufs);

        let expected = format!("--path-rename {}:{}", include_as[1], include_as[0]);
        assert_eq!(s.join(" "), expected);
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_iterate_over_files_in_folder() {
        let include_as = vec![
            "sometestlib/", "test/general/",
        ];
        let pathbufs = vec![
            PathBuf::from("test/general/end-to-end.bats"),
            PathBuf::from("test/general/usage.bats"),
        ];
        let s_vec = generate_split_arg_include_as(&include_as, pathbufs);
        let s = s_vec.join(" ");

        let expected1 = "--path-rename test/general/end-to-end.bats:sometestlib/end-to-end.bats";
        let expected2 = "--path-rename test/general/usage.bats:sometestlib/usage.bats";
        assert!(s.contains(expected1));
        assert!(s.contains(expected2));
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_work_for_nested_folders() {
        let include_as = vec![
            "sometestlib/", "test/",
        ];
        // there are a lot of paths, wont check for all of them
        let pathbufs = vec![
            PathBuf::from("test/general/"),
            PathBuf::from("test/general/end-to-end.bats"),
            PathBuf::from("test/general/usage.bats"),
            PathBuf::from("test/README.md"),
            PathBuf::from("test/splitout/end-to-end.bats"),
            PathBuf::from("test/splitout/"),
        ];
        let s_vec = generate_split_arg_include_as(&include_as, pathbufs);
        let s = s_vec.join(" ");
        println!("S: {}", s);

        let expected1 = "--path-rename test/general/end-to-end.bats:sometestlib/general/end-to-end.bats";
        let expected2 = "--path-rename test/README.md:sometestlib/README.md";
        let expected3 = "--path-rename test/splitout/end-to-end.bats:sometestlib/splitout/end-to-end.bats";
        assert!(s.contains(expected1));
        assert!(s.contains(expected2));
        assert!(s.contains(expected3));
    }

    #[test]
    fn generate_split_arg_include_as_should_work_for_root_rename() {
        let include_as = vec![
            "sometestlib/", " ",
        ];
        let pathbufs = vec![
            PathBuf::from("."),
            PathBuf::from("test/"),
            PathBuf::from("test/general/"),
            PathBuf::from("test/general/end-to-end.bats"),
            PathBuf::from("test/general/usage.bats"),
            PathBuf::from("test/README.md"),
            PathBuf::from("test/splitout/"),
            PathBuf::from("test/splitout/end-to-end.bats"),
            PathBuf::from("Cargo.toml"),
        ];
        let s_vec = generate_split_arg_include_as(&include_as, pathbufs);
        let s = s_vec.join(" ");

        let expected1 = "--path-rename test/general/end-to-end.bats:sometestlib/test/general/end-to-end.bats";
        let expected2 = "--path-rename test/README.md:sometestlib/test/README.md";
        let expected3 = "--path-rename Cargo.toml:sometestlib/Cargo.toml";
        // there are a lot of paths, wont check for all of them
        assert!(s.contains(expected1));
        assert!(s.contains(expected2));
        assert!(s.contains(expected3));
    }

    #[test]
    fn generate_split_arg_include_as_should_not_have_duplicates() {
        let include_as = vec![
            "sometestlib/", " ",
            "sometestlib/somesrc/", "src/",
        ];
        // this ones better to test with the real thing:
        let valid_files = get_commited_paths();
        let paths = generate_split_arg_include_as(&include_as, valid_files);
        println!("S: {}", paths.join(" "));
        let mut unique_paths = HashSet::new();
        // we go every other one because there are a lot of entries
        // that are just --path-rename, and that would get filtered out by the hash set
        // otherwise
        for p in paths.iter().skip(1).step_by(2) {
            assert!(!unique_paths.contains(&p));
            unique_paths.insert(p);
        }
    }

    #[test]
    fn generate_split_arg_include_as_should_not_contain_gitignored_files() {
        let include_as = vec![
            "sometestlib/", " ",
        ];
        let pathbufs = vec![
            // pretend that only Cargo.toml is not gitignored
            PathBuf::from("Cargo.toml"),
        ];
        let s_vec = generate_split_arg_include_as(&include_as, pathbufs);
        let s = s_vec.join(" ");
        println!("S: {}", s);

        let expected = "--path-rename Cargo.toml:sometestlib/Cargo.toml";
        let notexpected = "src/";

        // since src/ is a directory that is "ignored",
        // it shouldnt show up in the include_as_arg_str
        assert!(!s.contains(notexpected));
        assert!(s.contains(expected));
    }

    // // TODO: add this functionality. kinda annoying since it would need
    // // to exist across several methods...
    // //
    // // we cant do this for all include_as since it can be files or folders
    // // but in the case where user is bringing entire repo into a subdir
    // // (ie: include_as=("subdir" " ") we can detect this case
    // // so if the user forgets to place a trailing slash (which
    // // it needs, otherwise filter-repo will not work), then we should
    // // add one for them
    // #[test]
    // fn should_append_trailing_slash_if_missing_for_entire_repo_case() {
    //     let matches = ArgMatches::new();
    //     let mut runner = Runner::new(&matches);
    //     // ensure we dont actually run anything
    //     runner.dry_run = true;
    //     let mut repofile = RepoFile::new();
    //     // include_as has dest:src for split in
    //     // because its the reverse of the split out
    //     repofile.include_as = Some(vec![
    //         "locallib".into(), " ".into(),
    //     ]);
    //     runner.repo_file = repofile;
    //     runner = runner.generate_arg_strings();
    //     let include_arg_str_opt = runner.include_arg_str.clone();
    //     let include_as_arg_str_opt = runner.include_as_arg_str.clone();

    //     assert_eq!(include_as_arg_str_opt.unwrap(), "--path-rename  :locallib/");
    //     assert_eq!(include_arg_str_opt.unwrap(), "--path locallib/");
    // }
}
