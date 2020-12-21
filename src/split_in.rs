use std::convert::From;
use std::fs;
use std::path::Path;
use std::fmt::Display;
use std::{collections::HashSet, path::PathBuf};

use super::commands::INPUT_BRANCH_ARG;
use super::split::panic_if_array_invalid;
use super::split_out;
use super::git_helpers3;
use super::exec_helpers;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;
use super::repo_file::generate_repo_file_toml;
use super::die;
use super::repo_file;
use super::cli::MgtCommandSplit;
use super::core;
use super::topbase;


/// get a vector of paths of files that git knows about
/// starting from the root of the repo
pub fn get_commited_paths() -> Vec<PathBuf> {
    let data = exec_helpers::execute(&["git", "ls-files"]);
    let data = match data {
        Err(e) => die!("Failed to list git committed files: {}", e),
        Ok(d) => {
            if d.status != 0 {
                die!("Failed to list git committed files: {}", d.stderr);
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
        die!("Error reading dir recursively: {:?}", src);
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

pub fn run_split_in(cmd: &mut MgtCommandSplit) {
    let repo_file_path = if cmd.repo_file.len() < 1 {
        die!("Must provide repo path argument");
    } else {
        cmd.repo_file[0].clone()
    };

    let repo_file = repo_file::parse_repo_file_from_toml_path(&repo_file_path);
    run_split_in_from_repo_file(cmd, repo_file)
}

pub fn run_split_in_as(cmd: &mut MgtCommandSplit) {
    let include_as_src = match cmd.as_subdir {
        Some(ref s) => s,
        None => die!("Must provide an --as <subdirectory> option"),
    };
    // the field is called repo_file, but in split-in-as
    // its actually the repo_uri
    let repo_uri = match cmd.repo_file.len() {
        0 => die!("Must provide a git-repo-uri for split-in-as"),
        _ => cmd.repo_file[0].clone(),
    };
    let mut repo_file = RepoFile::new();
    repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    repo_file.remote_repo = Some(repo_uri.into());
    run_split_in_from_repo_file(cmd, repo_file);
}

pub fn run_split_in_from_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: RepoFile,
) {
    let mut repo_file = repo_file;
    core::verify_dependencies();
    validate_repo_file(cmd, &mut repo_file);
    core::go_to_repo_root();
    core::safe_to_proceed();
    let current_ref = core::get_current_ref();

    let orphan_branch_name = match cmd.output_branch {
        Some(ref s) => s,
        None => die!("Failed to parse a valid output branch. you may alternatively provide one with --output-branch <branch_name>"),
    };

    core::make_and_checkout_orphan_branch(
        orphan_branch_name,
        cmd.dry_run,
        cmd.verbose,
    );

    let remote_branch: Option<&str> = match &repo_file.remote_branch {
        Some(branch_name) => Some(branch_name.as_str()),
        None => None,
    };
    // if user provided a remote_branch name
    // on the command line, let that override what
    // is present in the repo file:
    let remote_branch = match split_out::get_remote_branch_from_args(cmd) {
        None => remote_branch,
        Some(new_remote_branch) => Some(new_remote_branch.as_str()),
    };

    core::populate_empty_branch_with_remote_commits(
        &repo_file,
        cmd.input_branch.as_deref(),
        remote_branch,
        cmd.num_commits,
        cmd.dry_run
    );

    let arg_strings = generate_arg_strings(&repo_file, cmd.dry_run, cmd.verbose);

    let log_p = if cmd.dry_run { "   # " } else { "" };
    if let Some(ref b) = cmd.output_branch {
        println!("{}Running filter commands on temporary branch: {}", log_p, b);
    }

    arg_strings.filter_in(
        &cmd.output_branch,
        cmd.dry_run,
        cmd.verbose
    );

    let res = if cmd.topbase.is_some() {
        println!("{}Topbasing", log_p);
        let should_add_branch_label = false;
        topbase::topbase(
            cmd.output_branch.clone().unwrap(),
            current_ref.unwrap(),
            cmd.dry_run,
            cmd.verbose,
            should_add_branch_label,
        )
    } else if cmd.rebase.is_some() {
        println!("{}Rebasing", log_p);
        core::rebase(current_ref, cmd.dry_run, cmd.verbose)
    } else {
        Ok(())
    };

    if let Ok(_) = res {
        println!("{}Success!", log_p);
    } else if let Err(e) = res {
        die!("{}", e);
    }
}

// TODO: use this when detecting a --gen-repo-file option
pub fn _generate_repo_file(
    repo_name: &str,
    repofile: &RepoFile
) -> Result<(), String> {
    let repo_file_path_str = format!("{}.rf", repo_name);
    let repo_file_path = std::path::PathBuf::from(&repo_file_path_str);
    if repo_file_path.exists() {
        let err_str = format!("{}.rf already exists", repo_name);
        return Err(err_str);
    }

    let repo_file_str = generate_repo_file_toml(repofile);
    match std::fs::write(repo_file_path_str, repo_file_str) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn validate_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: &mut RepoFile,
) {
    let input_branch = match cmd.input_branch {
        None => None,
        Some(ref branch_name) => {
            if ! git_helpers3::branch_exists(&branch_name) {
                die!("You specified an input branch of {}, but that branch was not found", branch_name);
            }
            Some(branch_name.clone())
        },
    };

    let missing_output_branch = cmd.output_branch.is_none();
    let missing_input_branch = cmd.input_branch.is_none();
    let missing_repo_name = repo_file.repo_name.is_none();
    let missing_remote_repo = repo_file.remote_repo.is_none();
    let missing_include_as = repo_file.include_as.is_none();
    let missing_include = repo_file.include.is_none();

    if missing_remote_repo && missing_input_branch && ! missing_output_branch {
        die!("Must provide either repo_name in your repofile, or specify a --{} argument", INPUT_BRANCH_ARG);
    }

    if missing_include && missing_include_as {
        die!("Must provide either include or include_as in your repofile");
    }

    if missing_repo_name && !missing_remote_repo && missing_output_branch {
        let output_branch_str = try_get_repo_name_from_remote_repo(
            repo_file.remote_repo.clone().unwrap()
        );
        repo_file.repo_name = Some(output_branch_str.clone());
        cmd.output_branch = Some(output_branch_str);
    } else if missing_output_branch && ! missing_repo_name {
        // make the repo_name the output branch name
        cmd.output_branch = Some(repo_file.repo_name.clone().unwrap());
    } else if missing_output_branch && ! missing_input_branch {
        // make the output_branch the name of the input_branch -reverse
        let output_branch_str = format!("{}-reverse", input_branch.clone().unwrap());
        cmd.output_branch = Some(output_branch_str);
    }

    panic_if_array_invalid(&repo_file.include, true, "include");
    panic_if_array_invalid(&repo_file.include_as, false, "include_as");
}

fn generate_arg_strings(
    repo_file: &RepoFile,
    dry_run: bool,
    verbose: bool,
) -> core::ArgStrings {
    let log_p = if dry_run { " #  " } else { "" };
    let include_as_arg_str = generate_split_out_arg_include_as(repo_file);
    let exclude_arg_str = generate_split_out_arg_exclude(repo_file);
    let include_arg_str = generate_split_out_arg_include(repo_file);

    if verbose {
        println!("{}include_arg_str: {}", log_p, include_arg_str.join(" "));
        println!("{}include_as_arg_str: {}", log_p, include_as_arg_str.join(" "));
        println!("{}exclude_arg_str: {}", log_p, exclude_arg_str.join(" "));
    }

    let include = if include_arg_str.len() != 0 {
        Some(include_arg_str)
    } else { None };

    let include_as = if include_as_arg_str.len() != 0 {
        Some(include_as_arg_str)
    } else { None };

    let exclude = if exclude_arg_str.len() != 0 {
        Some(exclude_arg_str)
    } else { None };

    core::ArgStrings {
        include_as,
        include,
        exclude,
    }
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
