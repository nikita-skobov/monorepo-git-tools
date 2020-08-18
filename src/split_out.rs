use std::env;
use clap::ArgMatches;

use super::commands::REPO_FILE_ARG;
use super::repo_file;
use super::repo_file::RepoFile;
use super::git_helpers;

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

pub fn run_split_out(matches: &ArgMatches) {
    // safe to unwrap because repo_file is a required argument
    let repo_file_name = matches.value_of(REPO_FILE_ARG).unwrap();
    println!("repo file: {}", repo_file_name);

    let mut repofile = repo_file::parse_repo_file(repo_file_name);
    // we validate the fields of the repo file
    // according to what split_out command wants it to be
    validate_repo_file(matches, &mut repofile);

    let current_dir = match env::current_dir() {
        Ok(pathbuf) => pathbuf,
        Err(_) => panic!("Failed to find your current directory. Cannot proceed"),
    };

    let (repo, repo_path) = git_helpers::get_repository_and_root_directory(&current_dir);
    println!("Found repo path: {}", repo_path.display());
    let include_arg_str = generate_split_out_arg_include(&repofile);
    println!("include_arg_str: {}", include_arg_str);
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
