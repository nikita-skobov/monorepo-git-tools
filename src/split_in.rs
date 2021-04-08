use std::convert::From;
use std::fs;
use std::path::Path;
use std::fmt::Display;
use std::{collections::HashSet, path::PathBuf};

use super::split_out;
use super::git_helpers3;
use super::exec_helpers;
use super::repo_file::RepoFile;
use super::repo_file::generate_repo_file_toml;
use super::die;
use super::repo_file;
use super::cli::MgtCommandSplit;
use super::core;
use super::verify;
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
    let is_split_in_as = false;
    run_split_in_from_repo_file(cmd, repo_file, is_split_in_as)
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
    let is_split_in_as = true;
    run_split_in_from_repo_file(cmd, repo_file, is_split_in_as);
}

pub fn run_split_in_from_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: RepoFile,
    split_in_as: bool,
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

    let log_p = if cmd.dry_run { "   # " } else { "" };
    if let Some(ref b) = cmd.output_branch {
        println!("{}Running filter commands on temporary branch: {}", log_p, b);
    }
    
    let filter_rules = generate_gitfilter_filterrules(&repo_file, cmd.verbose);
    core::perform_gitfilter(filter_rules, orphan_branch_name.clone(), cmd.dry_run, cmd.verbose);
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

    if let Err(e) = res {
        die!("{}", e);
    }

    // only allow repo file generation for split-in-as
    // subcommand. split-in already has a repo file...
    if split_in_as && cmd.generate_repo_file {
        let repo_file_name = match cmd.output_branch {
            Some(ref n) => n,
            None => "meta",
        };
        match generate_repo_file(repo_file_name, &repo_file) {
            Err(e) => die!("Failed to generate repo file: {}", e),
            Ok(_) => (),
        }
    }

    println!("{}Success!", log_p);
}

fn generate_gitfilter_filterrules(
    repo_file: &RepoFile,
    verbose: bool,
) -> gitfilter::filter::FilterRules {
    let mut file_ops = verify::get_vec_of_file_ops_with_order(&repo_file, false);
    let filter_rules = verify::make_filter_rules(&mut file_ops);
    filter_rules
}

pub fn generate_repo_file(
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
        die!("Must provide either repo_name in your repofile, or specify a --input-branch argument");
    }

    if missing_include && missing_include_as {
        die!("Must provide either include or include_as in your repofile");
    }

    if missing_repo_name && !missing_remote_repo && missing_output_branch {
        let output_branch_str = core::try_get_repo_name_from_remote_repo(
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

    core::panic_if_array_invalid(&repo_file.include, true, "include");
    core::panic_if_array_invalid(&repo_file.include_as, false, "include_as");
}
