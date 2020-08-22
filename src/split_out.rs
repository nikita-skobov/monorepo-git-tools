use std::env;
use std::path::PathBuf;
use clap::ArgMatches;

use super::commands::REPO_FILE_ARG;
use super::commands::DRY_RUN_ARG;
use super::commands::VERBOSE_ARG;
use super::repo_file;
use super::repo_file::RepoFile;
use super::git_helpers;
use super::exec_helpers;

pub struct Runner<'a> {
    matches: &'a ArgMatches<'a>,
    current_dir: PathBuf,
    // log prefix
    log_p: &'static str,
    pub dry_run: bool,
    pub verbose: bool,
    pub repo_file: RepoFile,
    pub repo_root_dir: PathBuf,
    pub repo: Option<git2::Repository>,
    pub include_arg_str: Option<String>,
    pub include_as_arg_str: Option<String>,
    pub exclude_arg_str: Option<String>,
}

impl<'a> Runner<'a> {
    pub fn new(matches: &'a ArgMatches) -> Runner<'a> {
        let is_verbose = matches.is_present(VERBOSE_ARG[0]);
        let is_dry_run = matches.is_present(DRY_RUN_ARG);
        Runner {
            matches: matches,
            dry_run: is_dry_run,
            verbose: is_verbose,
            repo_file: RepoFile::new(),
            current_dir: PathBuf::new(),
            repo: None,
            repo_root_dir: PathBuf::new(),
            include_arg_str: None,
            include_as_arg_str: None,
            exclude_arg_str: None,
            log_p: if is_dry_run { "   # " } else { "" },
        }
    }
    pub fn get_repo_file(mut self) -> Self {
        let repo_file_name = self.matches.value_of(REPO_FILE_ARG).unwrap();
        self.repo_file = repo_file::parse_repo_file(repo_file_name);
        if self.verbose {
            println!("{}repo file: {}", self.log_p, repo_file_name);
        }
        self
    }
    pub fn validate_repo_file(mut self) -> Self {
        validate_repo_file(self.matches, &mut self.repo_file);
        self
    }
    pub fn save_current_dir(mut self) -> Self {
        // save this for later, as well as to find the repository
        self.current_dir = match env::current_dir() {
            Ok(pathbuf) => pathbuf,
            Err(_) => panic!("Failed to find your current directory. Cannot proceed"),
        };
        if self.verbose {
            println!("{}saving current dir to return to later: {}", self.log_p, self.current_dir.display());
        }
        self
    }
    pub fn get_repository_from_current_dir(mut self) -> Self {
        let (repo, repo_path) = git_helpers::get_repository_and_root_directory(&self.current_dir);
        self.repo = Some(repo);
        self.repo_root_dir = repo_path;
        if self.verbose {
            println!("{}found repo path: {}", self.log_p, self.repo_root_dir.display());
        }
        self
    }
    pub fn change_to_repo_root(self) -> Self {
        if self.dry_run {
            println!("cd {}", self.repo_root_dir.display());
            return self;
        }
        if ! changed_to_repo_root(&self.repo_root_dir) {
            panic!("Failed to change to repository root: {:?}", &self.repo_root_dir);
        }
        if self.verbose {
            println!("{}changed to repository root {}", self.log_p, self.repo_root_dir.display());
        }
        self
    }
    pub fn generate_arg_strings(mut self) -> Self {
        let include_arg_str = generate_split_out_arg_include(&self.repo_file);
        let include_as_arg_str = generate_split_out_arg_include_as(&self.repo_file);
        let exclude_arg_str = generate_split_out_arg_exclude(&self.repo_file);
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
    pub fn make_and_checkout_output_branch(self) -> Self {
        if self.dry_run {
            println!("git checkout -b {}", self.repo_file.repo_name.clone().unwrap());
            return self;
        }

        let output_branch_name = self.repo_file.repo_name.clone().unwrap();
        match self.repo {
            Some(ref r) => {
                let success = git_helpers::make_new_branch_from_head_and_checkout(
                    r,
                    output_branch_name.as_str(),
                ).is_ok();
                if ! success {
                    panic!("Failed to checkout new branch");
                }
            },
            _ => panic!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out new branch {}", self.log_p, output_branch_name);
        }

        self
    }
    // panic if all dependencies are not met
    pub fn verify_dependencies(self) -> Self {
        if ! exec_helpers::executed_successfully(&["git", "--version"]) {
            panic!("Failed to run. Missing dependency 'git'");
        }
        if ! exec_helpers::executed_successfully(&["git", "filter-repo", "--version"]) {
            panic!("Failed to run. Missing dependency 'git-filter-repo'");
        }
        self
    }
    pub fn run_filter(self, arg_vec: Vec<&str>, verbose_log: &str) -> Self {
        if self.dry_run {
            println!("{}", arg_vec.join(" "));
            return self
        }
        if self.verbose {
            println!("{}", verbose_log);
        }
        if ! exec_helpers::executed_successfully(&arg_vec) {
            panic!("Failed to execute: \"{}\"", arg_vec.join(" "));
        }

        self
    }

    pub fn filter_include(self) -> Self {
        if self.include_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.repo_file.repo_name.clone().unwrap();
        let include_arg_str_opt = self.include_arg_str.clone();
        let include_arg_str = include_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            include_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering include")
    }
    pub fn filter_include_as(self) -> Self {
        if self.include_as_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.repo_file.repo_name.clone().unwrap();
        let include_as_arg_str_opt = self.include_as_arg_str.clone();
        let include_as_arg_str = include_as_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            include_as_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering include_as")
    }
    pub fn filter_exclude(self) -> Self {
        if self.exclude_arg_str.is_none() {
            // dont run filter if this arg was not provided
            return self;
        }
        let output_branch_name = self.repo_file.repo_name.clone().unwrap();
        let exclude_arg_str_opt = self.exclude_arg_str.clone();
        let exclude_arg_str = exclude_arg_str_opt.unwrap();
        let arg_vec = generate_filter_arg_vec(
            exclude_arg_str.as_str(),
            output_branch_name.as_str(),
        );

        self.run_filter(arg_vec, "Filtering exclude")
    }
}

pub fn generate_filter_arg_vec<'a>(
    arg_str: &'a str,
    output_branch: &'a str
) -> Vec<&'a str> {
    let mut arg_vec = vec!["git", "filter-repo"];
    for arg in arg_str.split_whitespace() {
        arg_vec.push(arg);
    }
    arg_vec.push("--refs");
    arg_vec.push(&output_branch);
    arg_vec.push("--force");

    arg_vec
}

fn get_string_after_last_slash(s: String) -> String {
    let mut pieces = s.rsplit('/');
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}

fn get_string_before_first_dot(s: String) -> String {
    let mut pieces = s.split('.');
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}

pub fn is_valid_remote_repo(remote_repo: &String) -> bool {
    // TODO:
    // need to check for if it matches a regex like a server ip
    // like 192.168.1.1, or user@server.com:/gitpath
    return remote_repo.starts_with("ssh://") ||
    remote_repo.starts_with("git://") ||
    remote_repo.starts_with("http://") ||
    remote_repo.starts_with("https://") ||
    remote_repo.starts_with("ftp://") ||
    remote_repo.starts_with("sftp://") ||
    remote_repo.starts_with("file://") ||
    remote_repo.starts_with(".") ||
    remote_repo.starts_with("/");
}

// try to parse the remote repo
pub fn try_get_repo_name_from_remote_repo(remote_repo: String) -> String {
    let mut out_str = remote_repo.clone().trim_end().to_string();
    if !is_valid_remote_repo(&remote_repo) {
        out_str = "".into();
    }
    if out_str.ends_with("/") {
        out_str.pop();
    }
    if !out_str.contains("/") {
        out_str = "".into();
    }
    out_str = get_string_after_last_slash(out_str);
    out_str = get_string_before_first_dot(out_str);

    if out_str == "" {
        panic!("Failed to parse repo_name from remote_repo: {}", remote_repo);
    }

    return out_str;
}

// works for include, or include_as
// the variable is valid if it is a single item,
// or if it is multiple items, it is valid if it has an even length
pub fn include_var_valid(var: &Vec<String>, can_be_single: bool) -> bool {
    let vlen = var.len();
    if vlen == 1 && can_be_single {
        return true;
    }
    if vlen >= 1 && vlen % 2 == 0 {
        return true;
    }
    return false;
}

pub fn panic_if_array_invalid(var: &Option<Vec<String>>, can_be_single: bool, varname: &str) {
    match var {
        Some(v) => {
            if ! include_var_valid(&v, can_be_single) {
                panic!("{} is invalid. Must be either a single string, or an even length array of strings", varname);
            }
        },
        _ => (),
    };
}

pub fn validate_repo_file(matches: &ArgMatches, repofile: &mut RepoFile) {
    let missing_repo_name = repofile.repo_name.is_none();
    let missing_remote_repo = repofile.remote_repo.is_none();
    let missing_include_as = repofile.include_as.is_none();
    let missing_include = repofile.include.is_none();

    if missing_remote_repo && missing_repo_name {
        panic!("Must provide either repo_name or remote_repo in your repofile");
    }

    if missing_include && missing_include_as {
        panic!("Must provide either include or include_as in your repofile");
    }

    if missing_repo_name && !missing_remote_repo {
        repofile.repo_name = Some(try_get_repo_name_from_remote_repo(
            repofile.remote_repo.clone().unwrap()
        ));
    }

    panic_if_array_invalid(&repofile.include, true, "include");
    panic_if_array_invalid(&repofile.include_as, false, "include_as");
}

// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
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

    // sources are the even indexed elements, dest are the odd
    let sources = include_as.iter().skip(0).step_by(2);
    let destinations = include_as.iter().skip(1).step_by(2);
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

pub fn generate_split_out_arg_exclude(repofile: &RepoFile) -> String {
    let start_with: String = "--invert-paths --path ".into();
    match &repofile.exclude {
        Some(v) => format!("{}{}", start_with.clone(), v.join(" --path ")),
        None => "".to_string(),
    }
}

pub fn changed_to_repo_root(repo_root: &PathBuf) -> bool {
    match env::set_current_dir(repo_root) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn run_split_out(matches: &ArgMatches) {
    Runner::new(matches)
        .get_repo_file()
        .verify_dependencies()
        .validate_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .change_to_repo_root()
        .generate_arg_strings()
        .make_and_checkout_output_branch()
        .filter_include()
        .filter_exclude()
        .filter_include_as();
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo")]
    fn should_panic_if_no_repo_name_or_remote_repo() {
        let mut repofile = RepoFile::new();
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    fn should_generate_include_args_properly() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "abc".into(), "abc-remote".into(),
            "xyz".into(), "xyz-remote".into(),
        ]);
        repofile.include = Some(vec!["123".into()]);
        let filter_args = generate_split_out_arg_include(&repofile);
        assert_eq!(filter_args, "--path 123 --path abc --path xyz");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_one_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args, "--invert-paths --path one");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_multiple_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(), "two".into(), "three".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args, "--invert-paths --path one --path two --path three");
    }

    // not sure how to test this. I want to test if
    // println was called with certain strings (because dry-run was set true
    // that means it will println instead of running the command)
    // but i dont think you can do that in rust
    // #[test]
    // fn should_not_run_filter_exclude_if_no_exclude_provided() {
    //     let matches = ArgMatches::new();
    //     let mut runner = Runner::new(&matches);
    //     // ensure we dont actually run anything
    //     runner.dry_run = true;
    //     let repofile = RepoFile::new();
    //     runner.repo_file = repofile;
    //     runner = runner.generate_arg_strings();
    //     let exclude_arg_str = runner.exclude_arg_str.clone().unwrap();
    //     assert_eq!(exclude_arg_str, "".to_string());
    // }

    #[test]
    fn should_generate_empty_strings_for_generating_args_of_none() {
        let mut repofile = RepoFile::new();
        repofile.exclude = None;
        repofile.include = None;
        repofile.include_as = None;

        let filter_exclude_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_exclude_args, "");

        let filter_include_args = generate_split_out_arg_include(&repofile);
        assert_eq!(filter_include_args, "");

        let filter_include_as_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_include_as_args, "");
    }

    // similarly to the above test, if the actual generate_split_out_arg_include_as
    // returns "", then we want the generate_arg_strings method to
    // set the fields to None
    #[test]
    fn should_set_arg_strs_to_none_if_empty_string() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        // ensure we dont actually run anything
        runner.dry_run = true;
        let mut repofile = RepoFile::new();
        repofile.exclude = None;
        repofile.include = None;
        repofile.include_as = None;
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        let exclude_arg_str_opt = runner.exclude_arg_str.clone();
        let include_arg_str_opt = runner.include_arg_str.clone();
        let include_as_arg_str_opt = runner.include_as_arg_str.clone();

        assert_eq!(exclude_arg_str_opt, None);
        assert_eq!(include_arg_str_opt, None);
        assert_eq!(include_as_arg_str_opt, None);
    }

    #[test]
    fn should_generate_split_out_arg_include_as_properly() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "abc-src".into(), "abc-dest".into(),
            "xyz-src".into(), "xyz-dest".into(),
        ]);
        let filter_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_args, "--path-rename abc-src:abc-dest --path-rename xyz-src:xyz-dest");
    }

    // if include_as is None, it shouldnt fail, but rather
    // just return an empty string
    #[test]
    fn gen_split_out_arg_include_as_should_not_fail_if_no_include_as() {
        let repofile = RepoFile::new();
        let filter_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_args, "");
    }

    #[test]
    #[should_panic(expected = "Must provide either include")]
    fn should_panic_if_no_include_or_include_as() {
        let mut repofile = RepoFile::new();
        repofile.repo_name = Some("reponame".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    #[should_panic(expected = "Failed to parse repo_name from remote_repo")]
    fn should_panic_if_failed_to_parse_repo_name_from_remote_repo() {
        let mut repofile = RepoFile::new();
        repofile.include = Some(vec!["sdsa".into()]);
        repofile.remote_repo = Some("badurl".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    fn should_parse_repo_name_from_remote_repo_if_valid_path() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec!["dsadsa".into(), "dsadsa".into()]);
        repofile.remote_repo = Some("./path/to/reponame".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
        assert_eq!(repofile.repo_name.unwrap(), "reponame".to_string());
    }

    #[test]
    fn should_parse_repo_name_from_remote_repo_if_valid_url() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec!["dsadsa".into(), "dsadsa".into()]);
        repofile.remote_repo = Some("https://website.com/path/to/reponame.git".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
        assert_eq!(repofile.repo_name.unwrap(), "reponame".to_string());
    }

    #[test]
    #[should_panic(expected = "Must be either a single string, or")]
    fn should_panic_if_include_is_odd() {
        let mut repofile = RepoFile::new();
        // vec of 3 strings:
        repofile.include = Some(vec!["dsadsa".into(), "dsadsa".into(), "dsadsa".into()]);
        repofile.repo_name = Some("dsadsa".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }

    #[test]
    #[should_panic(expected = "Must be either a single string, or")]
    fn should_panic_if_include_as_is_single_string() {
        let mut repofile = RepoFile::new();
        // single string should not be valid for include_as
        repofile.include_as = Some(vec!["dsadsa".into()]);
        repofile.repo_name = Some("dsadsa".into());
        let argmatches = ArgMatches::new();
        validate_repo_file(&argmatches, &mut repofile);
    }
}
