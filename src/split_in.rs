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
use std::path::PathBuf;

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

        if missing_remote_repo && missing_input_branch {
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
        } else if ! missing_repo_name {
            // make the repo_name the output branch name
            self.output_branch = Some(self.repo_file.repo_name.clone().unwrap());
        } else if ! missing_input_branch {
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
            println!("{}include_arg_str: {}", self.log_p, include_arg_str);
            println!("{}include_as_arg_str: {}", self.log_p, include_as_arg_str);
            println!("{}exclude_arg_str: {}", self.log_p, exclude_arg_str);
        }

        if include_arg_str != "" {
            self.include_arg_str = Some(include_arg_str);
        }
        if include_as_arg_str != "" {
            self.include_as_arg_str = Some(include_as_arg_str);
        }
        if exclude_arg_str != "" {
            self.exclude_arg_str = Some(exclude_arg_str);
        }

        self
    }

    fn make_and_checkout_output_branch(mut self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();

        self.make_and_checkout_orphan_branch(output_branch_name.as_str())
    }
}

// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
// include gets taken as is, but for include_as, we only care about
// what the destination is because we will run
// the include filter after we rename, so we want the renamed versions
pub fn generate_split_out_arg_include(repofile: &RepoFile) -> String {
    let start_with: String = "--path ".into();
    let include_str = match &repofile.include {
        Some(v) => format!("{}{}", start_with.clone(), v.join(" --path ")),
        None => "".to_string(),
    };

    // include_as is more difficult because the indices matter
    // for splitting out, the even indices are the local
    // paths, so those are the ones we want to include
    let include_as_str = match &repofile.include_as {
        Some(v) => format!("{}{}",
            start_with.clone(),
            v.iter().step_by(2)
                .cloned().collect::<Vec<String>>()
                .join(" --path "),
        ),
        None => "".to_string(),
    };

    // include a space between them if include_str isnt empty
    let seperator: String = if include_str.is_empty() {
        "".into()
    } else {
        " ".into()
    };

    format!("{}{}{}", include_str, seperator, include_as_str)
}


// iterate over the include_as variable, and generate a
// string of args that can be passed to git-filter-repo
pub fn generate_split_out_arg_include_as(repofile: &RepoFile) -> String {
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        return "".into();
    };

    // for split-in src/dest is reversed from spit-out
    // sources are the odd indexed elements, dest are the even
    let sources = include_as.iter().skip(1).step_by(2);
    let destinations = include_as.iter().skip(0).step_by(2);
    assert_eq!(sources.len(), destinations.len());

    let pairs = sources.zip(destinations);
    // pairs is a vec of tuples: (src, dest)
    // when mapping, x.0 is src, x.1 is dest
    format!("--path-rename {}",
        pairs.map(|x| format!("{}:{}", x.0, x.1))
            .collect::<Vec<String>>()
            .join(" --path-rename ")
    )
}

// given a vec of include_as pairs,
// iterate over the odd-indexed elements.
// if the path is a file include it as is
// if the path is a folder, iterate over the files in that folder
// recursively, and add a --path-rename file:file instead of folder:folder
pub fn generate_split_arg_include_as<T: AsRef<str> + AsRef<Path> + Display>(
    include_as: &[T]
) -> String {
    let sources = include_as.iter().skip(1).step_by(2);
    let destinations = include_as.iter().skip(0).step_by(2);

    let pairs = sources.zip(destinations);

    let mut out_str: String = "".into();
    for (src, dest) in pairs {
        let src_str: &str = src.as_ref();
        let src_pb = PathBuf::from(src_str);

        out_str = format!("{}{}", out_str, match src_pb.is_file() {
            // files can keep their existing mapping
            true => format!("--path-rename {}:{}", src, dest),
            // but for folders, we should iterate recursively into the
            // folder and add every file mapping explicitly
            false => gen_include_as_arg_files_from_folder(dest.as_ref(), src_pb),
        });
    }

    out_str
}

pub fn get_files_recursively(dir: PathBuf, file_vec: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let dir_entry = entry?;
        let dir_path = dir_entry.path();
        if dir_path.is_file() {
            file_vec.push(dir_path.to_path_buf());
        } else {
            get_files_recursively(dir_path.to_path_buf(), file_vec)?;
        }
    }
    Ok(())
}

pub fn gen_include_as_arg_files_from_folder(dest: &str, src: PathBuf) -> String {
    let mut file_vec = vec![];
    let all_files_recursively = get_files_recursively(src.clone(), &mut file_vec);
    if all_files_recursively.is_err() {
        panic!("Error reading dir recursively: {:?}", src);
    }

    let mut out_str: String = "".into();
    for f in file_vec {
        let src_str = &src.to_str().unwrap();
        // we will replace the original src prefix
        // with the provided dest prefix, so strip the current prefix here
        let f_str = f.strip_prefix(&src).unwrap().to_str().unwrap();
        let new_src = format!("{}{}", src_str, f_str);
        let new_dest = format!("{}{}", dest, f_str);

        // formatting: if there is a previous entry, add
        // a space between prev entry and this next one
        if out_str.len() > 0 {
            out_str.push(' ');
        }
        out_str = format!("{}{}",
            out_str,
            format!("--path-rename {}:{}", new_src, new_dest),
        );
    }

    out_str
}

pub fn generate_split_out_arg_exclude(repofile: &RepoFile) -> String {
    let start_with: String = "--invert-paths --path ".into();
    match &repofile.exclude {
        Some(v) => format!("{}{}", start_with.clone(), v.join(" --path ")),
        None => "".to_string(),
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
    let runner = runner
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
        runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        println!("{}Rebasing", log_p);
        runner.get_repository_from_current_dir().rebase();
    }

    println!("{}Success!", log_p);
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
    let runner = runner
        .filter_exclude()
        .filter_include_as()
        .filter_include();

    if runner.should_topbase {
        println!("{}Topbasing", log_p);
        use super::topbase::Topbase;
        runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        println!("{}Rebasing", log_p);
        runner.get_repository_from_current_dir().rebase();
    }

    println!("{}Success!", log_p);
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo_name in your repofile, or specify a")]
    fn should_panic_if_missing_input_branch_and_remote_repo() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let repofile = RepoFile::new();
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }

    #[test]
    fn should_format_include_as_correctly() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "path/will/be/created/".into(), " ".into(),
        ]);
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        assert_eq!(runner.include_as_arg_str.unwrap(), "--path-rename  :path/will/be/created/");
    }

    // even if user only provides include_as, that is just for renaming
    // we also need to translate that to an include step so that
    // we include only the things the user specifies, otherwise
    // we would just have renamed some folders/files
    #[test]
    fn generate_arg_strings_should_make_an_include_from_include_as() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        // ensure we dont actually run anything
        runner.dry_run = true;
        let mut repofile = RepoFile::new();
        // include_as has dest:src for split in
        // because its the reverse of the split out
        repofile.include_as = Some(vec![
            "locallib/".into(), "lib/".into(),
        ]);
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        let include_arg_str_opt = runner.include_arg_str.clone();
        let include_as_arg_str_opt = runner.include_as_arg_str.clone();

        assert_eq!(include_as_arg_str_opt.unwrap(), "--path-rename lib/:locallib/");
        assert_eq!(include_arg_str_opt.unwrap(), "--path locallib/");
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_return_the_mapping_as_is_if_src_is_file() {
        let include_as = vec![
            "src/lib/Cargo.toml", "Cargo.toml",
        ];
        let s = generate_split_arg_include_as(&include_as);

        let expected = format!("--path-rename {}:{}", include_as[1], include_as[0]);
        assert_eq!(s, expected);
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_iterate_over_files_in_folder() {
        let include_as = vec![
            "sometestlib/", "test/general/",
        ];
        let s = generate_split_arg_include_as(&include_as);

        let expected1 = "--path-rename test/general/end-to-end.bats:sometestlib/end-to-end.bats";
        let expected2 = "--path-rename test/general/usage.bats:sometestlib/usage.bats";
        let expected = format!("{} {}", expected1, expected2);
        assert_eq!(s, expected);
    }

    // this test requires reading files/folders from the root of the
    // repo. make sure when you run tests, you are at the root of the repo
    #[test]
    fn generate_split_arg_include_as_should_work_for_nested_folders() {
        let include_as = vec![
            "sometestlib/", "test/",
        ];
        let s = generate_split_arg_include_as(&include_as);

        let expected1 = "--path-rename test/general/end-to-end.bats:sometestlib/general/end-to-end.bats";
        let expected2 = "--path-rename test/README.md:sometestlib/README.md";
        let expected3 = "--path-rename test/splitout/end-to-end.bats:sometestlib/splitout/end-to-end.bats";
        // there are a lot of paths, wont check for all of them
        assert!(s.contains(expected1));
        assert!(s.contains(expected2));
        assert!(s.contains(expected3));
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
