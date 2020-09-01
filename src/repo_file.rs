use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq)]
pub struct RepoFile {
    /// ## repo_name
    /// The name of the remote repository <br/>
    /// This will be the branch name when mgt creates a temporary branch. <br/>
    /// Required only if `remote_repo` is not specified.
    pub repo_name: Option<String>,
    /// ## remote_repo
    /// A valid git repo uri. Can be a local file location, remote url, ssh url, etc. <br/>
    /// For `split-in` the git history of `remote_repo` is rewritten to match this local repo's history. <br/>
    /// For `split-out` the git history of this local repository is rewritten to match the `remote_repo`. <br/>
    /// Required for `split-in`, only required for `split-out` if using `--topbase` or `--rebase`.
    pub remote_repo: Option<String>,
    /// A name of a branch available on the `remote_repo`. By default `split-in` (and `split-out` if 
    /// using `--topbase` or `--rebase`) use the HEAD of the `remote_repo`, but you can specify a specific
    /// branch to use instead.
    /// Optional.
    /// ## remote_branch
    pub remote_branch: Option<String>,
    /// A list of paths where even-indexed paths are the sources, and odd-indexed paths are the destinations. <br/>
    /// The source is a path to a file/folder in this local repository, and the destination is
    /// a path to a file/folder in the remote repository. <br/>
    /// This is so that you can use the same `repo_file` for both splitting in and out.
    /// 
    /// Examples:
    /// ```
    /// include_as=("my_markdown_notes/002-notes-on-this-thing.md" "README.md")
    /// ```
    /// When running `split-out` this will rewrite the users repository
    /// and only keep the file: `my_markdown_notes/002-notes-on-this-thing.md`, however
    /// when it rewrites the history, it will also rename the file to be `README.md`.
    /// When running `split-in` this will take the `README.md` file from the `remote_repo`
    /// and rename it to be `my_markdown_notes/002-notes-on-this-thing.md`
    /// ```
    /// include_as=(
    ///     "lib/file1.txt" "file1.txt"
    ///     "lib/project/" " "
    /// )
    /// ``` 
    /// For `split-out` this will rename the local repository's `lib/file1.txt` to just `file1.txt`
    /// and it will take the entire folder `lib/project/` and make that the root of the split out repository.
    /// NOTE that when specifying directories, you MUST include a trailing slash. And if you wish to make a subdirectory
    /// the root of the split repository, the correct syntax is a single empty space: `" "`.
    /// ## include_as
    pub include_as: Option<Vec<String>>,
    /// A list of paths to include. Unlike `include_as`, this does not allow for renaming.
    /// There is no source/destination here, it is just a list of paths to keep exactly as they are.
    ///
    /// Examples:
    /// ```
    /// include=(
    ///    "README.md"
    ///    "LICENSE"
    /// )
    /// ```
    /// This will only take the `README.md` and `LICENSE` files at the root level, and ignore everything else.
    /// ```
    /// include="lib/"
    /// include=("lib/")
    /// ```
    /// Both of the above are valid. `include` can be a single string if you only have one path to include.
    /// ## include
    pub include: Option<Vec<String>>,
    /// A list of paths to exclude. This is useful if you want a folder, but don't want some of the
    /// subfolders.
    ///
    /// Examples:
    /// ```
    /// include="lib/"
    /// exclude=("lib/private/" "lib/README.md")
    /// ```
    /// For `split-in` this will take the entirety of the `lib/` folder, but will not take `lib/README.md` and
    /// will not take the entire subfolder `lib/private/`. Note that `exclude` does not make sense for both `split-out`
    /// and `split-in`. In the above example, if you use this same `repo_file` again to `split-out` your changes,
    /// you do not have a `lib/private` or a `lib/README.md`, so this `exclude` statement will not do anything.
    /// This means you can specify both local paths to exclude and remote paths to exclude:
    /// ```
    /// exclude=(
    ///    "localfile.txt"
    ///    "remotefile.txt"
    /// )
    /// ```
    /// If your local repository has a `localfile.txt` then `split-out` will not include it, and `split-out` will do
    /// nothing about the `remotefile.txt` (because there isn't one).<br/>
    /// If the remote repository has a `remotefile.txt` then that file will be excluded when running `split-in`. <br/>
    /// NOTE: in the future there might be an `exclude_local` and `exclude_remote` to avoid these ambiguities.
    /// ## exclude
    pub exclude: Option<Vec<String>>,
}

impl RepoFile {
    pub fn new() -> RepoFile {
        RepoFile {
            repo_name: None,
            remote_repo: None,
            remote_branch: None,
            include: None,
            include_as: None,
            exclude: None,
        }
    }
}

const RFVN_REMOTE_BRANCH: &'static str = "remote_branch";
const RFVN_REPO_NAME: &'static str = "repo_name";
const RFVN_REMOTE_REPO: &'static str = "remote_repo";
const RFVN_INCLUDE_AS: &'static str = "include_as";
const RFVN_INCLUDE: &'static str = "include";
const RFVN_EXCLUDE: &'static str = "exclude";

#[derive(Clone, PartialEq)]
pub enum RepoFileVariableName {
    VarRemoteRepo,
    VarRepoName,
    VarRemoteBranch,
    VarIncludeAs,
    VarExclude,
    VarInclude,
    VarUnknown,
}
use RepoFileVariableName::*;
impl From<RepoFileVariableName> for &'static str {
    fn from(original: RepoFileVariableName) -> &'static str {
        match original {
            VarRepoName => RFVN_REPO_NAME,
            VarRemoteRepo => RFVN_REMOTE_REPO,
            VarRemoteBranch => RFVN_REMOTE_BRANCH,
            VarIncludeAs => RFVN_INCLUDE_AS,
            VarInclude => RFVN_INCLUDE,
            VarExclude => RFVN_EXCLUDE,
            VarUnknown => "",
        }
    }
}
impl From<String> for RepoFileVariableName {
    fn from(value: String) -> RepoFileVariableName {
        match value.as_str() {
            RFVN_REMOTE_BRANCH => VarRemoteBranch,
            RFVN_REPO_NAME => VarRepoName,
            RFVN_INCLUDE => VarInclude,
            RFVN_EXCLUDE => VarExclude,
            RFVN_REMOTE_REPO => VarRemoteRepo,
            RFVN_INCLUDE_AS => VarIncludeAs,
            _ => VarUnknown,
        }
    }
}


#[derive(PartialEq)]
enum RepoFileVariableType {
    TypeString,
    TypeArray,
    TypeUnknown,
}
use RepoFileVariableType::*;

struct RepoFileVariable {
    name: RepoFileVariableName,
    value: Vec<String>,
    complete: bool,
    var_type: RepoFileVariableType,
}

fn get_variable_name(text: &String) -> String {
    let equals_index = text.find("=").unwrap();
    let str_before_equals: String = text.chars().take(equals_index).collect();
    return str_before_equals.replace(" ", "");
}

// a variable is being assigned. it is expected
// that the next character after the equals is either
// a quote, or an opening parentheses
fn char_after_equals(text: &String) -> char {
    let equals_index = text.find("=").unwrap();
    for c in text.chars().skip(equals_index + 1) {
        if !c.is_whitespace() {
            return c;
        }
    }
    return ' ';
}

fn get_all_strings(text: &String) -> Option<Vec<String>> {
    let mut strings: Vec<String> = vec![];
    let mut current_string = String::from("");
    let mut should_append_string = false;
    for c in text.chars() {
        if c == '#' {
            break;
        }
        // found the start of a string
        if c == '"' && current_string == "" {
            should_append_string = true;
        }
        if c != '"' && should_append_string {
            current_string.push(c);
        }
        if c == '"' && current_string != "" {
            strings.push(current_string.clone());
            current_string = String::from("");
            should_append_string = false;
        }
    }

    // when we exit loop, current string should be empty
    // if its not, then it was a failure to parse and we should
    // return empty vector
    if current_string != "" {
        return None;
    }

    return Some(strings);
}

fn is_end_of_array(text: &String) -> bool {
    let mut is_end = false;
    for c in text.chars() {
        if c == '#' {
            break;
        }
        if c == ')' {
            is_end = true;
            break;
        }
    }
    return is_end;
}

fn parse_variable(variable: &mut RepoFileVariable, text: &String, line_num: usize) {
    if variable.name == VarUnknown && text.contains("=") {
        // variable is empty, and this line
        // contains an equal sign, so we assume the variable
        // is being created
        variable.name = get_variable_name(&text).into();
        variable.var_type = match char_after_equals(&text) {
            '(' => TypeArray,
            '"' => TypeString,
             _  => TypeUnknown,
        };
    }

    if variable.name == VarUnknown {
        panic!("Invalid variable name found on line {}:\n\"{}\"", line_num, text);
    }

    if variable.var_type == TypeUnknown {
        panic!("Failed to parse line {}:\n\"{}\"", line_num, text);
    }

    let strings = get_all_strings(&text);
    if let None = strings {
        panic!("Failed to parse variable at line {}:\n\"{}\"", line_num, text);
    }

    match variable.var_type {
        // easiest case to parse. just get everything between the quotes
        // there should only be one string
        TypeString => {
            variable.value = vec![strings.unwrap()[0].clone()];
            variable.complete = true;
        },
        // harder to parse. add all the strings we found
        // and then only mark it complete if we also found the
        // end of the array
        TypeArray => {
            variable.value.append(&mut strings.unwrap());
            variable.complete = is_end_of_array(&text);
        },
        // we already checked for TypeUnknown above
        _ => (),
    }
}

fn add_variable_to_repo_file(repofile: &mut RepoFile, variable: &mut RepoFileVariable) {
    match variable.name {
        VarRemoteRepo => repofile.remote_repo = Some(variable.value[0].clone()),
        VarIncludeAs => repofile.include_as = Some(variable.value.clone()),
        VarExclude => repofile.exclude = Some(variable.value.clone()),
        VarInclude => repofile.include = Some(variable.value.clone()),
        VarRepoName => repofile.repo_name = Some(variable.value[0].clone()),
        VarRemoteBranch => repofile.remote_branch = Some(variable.value[0].clone()),
        _ => (),
    }

    variable.name = VarUnknown;
    variable.value = vec![];
}

// returns true if line is not a full line comment
// and if line is not entirely whitespace
fn should_parse_line(text: &String) -> bool {
    let mut is_entirely_whitespace = true;
    let mut is_full_line_comment = false;
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        } else {
            is_entirely_whitespace = false;
        }

        if c == '#' {
            is_full_line_comment = true;
        }
        break;
    }

    return !is_full_line_comment && !is_entirely_whitespace;
}

pub fn parse_repo_file(filename: &str) -> RepoFile {
    let repo_file_path = Path::new(filename);
    if !repo_file_path.exists() {
        panic!("Failed to find repo_file: {}", filename);
    }

    let file = File::open(repo_file_path);
    if let Err(file_error) = file {
        panic!("Failed to open file: {}, {}", filename, file_error);
    }

    let file_contents = file.unwrap();
    let reader = BufReader::new(file_contents);
    let lines: Vec<String> = reader.lines().map(|x| x.unwrap()).collect();
    return parse_repo_file_from_lines(lines);
}

pub fn parse_repo_file_from_lines(lines: Vec<String>) -> RepoFile {
    let mut repofile_obj = RepoFile::new();

    // this will be modified by the parse_variable func above
    // everytime this variable is "complete", it will be added
    // to the RepoFile struct
    let mut current_variable = RepoFileVariable{
        name: VarUnknown,
        value: vec![],
        complete: false,
        var_type: TypeUnknown,
    };

    for (line_num, line) in lines.iter().enumerate() {
        if should_parse_line(&line) {
            parse_variable(&mut current_variable, line, line_num);
        }

        if current_variable.complete {
            add_variable_to_repo_file(&mut repofile_obj, &mut current_variable);
            current_variable.complete = false;
        }
    }
    return repofile_obj;
}

#[cfg(test)]
mod test {
    use super::RepoFile;
    use super::parse_repo_file_from_lines;

    #[test]
    #[should_panic(expected = "Invalid variable name")]
    fn should_panic_if_finds_unknown_var() {
        let lines: Vec<String> = vec![
            "my_unknown_var=something".into()
        ];
        parse_repo_file_from_lines(lines);
    }

    #[test]
    #[should_panic(expected = "Failed to parse line")]
    fn should_panic_if_variable_type_unknown() {
        let lines: Vec<String> = vec![
            "remote_repo=something".into()
        ];
        parse_repo_file_from_lines(lines);
    }

    #[test]
    fn should_handle_big_space_in_array() {
        let lines: Vec<String> = vec![
            "include_as=(\"abc\" \"xyz\"".into(),
            "".into(),
            "\t\t  \t".into(),
            "\n\n    \n\t\t".into(),
            ")".into(),
            " ".into(),
            "exclude=\"yyy\"".into(),
        ];
        let mut expectedrepofileobj = RepoFile::new();
        expectedrepofileobj.include_as = Some(vec![
            "abc".into(), "xyz".into(),
        ]);
        expectedrepofileobj.exclude = Some(vec!["yyy".into()]);
        let repofileobj = parse_repo_file_from_lines(lines);
        assert_eq!(expectedrepofileobj, repofileobj);
    }

    #[test]
    fn should_return_repo_file_obj() {
        let lines: Vec<String> = vec![
            "remote_repo=\"something\"".into(),
            "include_as=(".into(),
            "    \"one\"".into(),
            "    \"two\" \"three\"".into(),
            "              )".into(),
            "exclude=(\"abc\")".into(),
            "    include=(\"xyz\" \"qqq\" \"www\")".into(),
        ];
        let mut expectedrepofileobj = RepoFile::new();
        expectedrepofileobj.remote_repo = Some("something".into());
        expectedrepofileobj.include_as = Some(vec![
            "one".into(), "two".into(), "three".into()
        ]);
        expectedrepofileobj.exclude = Some(vec!["abc".into()]);
        expectedrepofileobj.include = Some(vec![
            "xyz".into(), "qqq".into(), "www".into(),
        ]);
        let repofileobj = parse_repo_file_from_lines(lines);
        assert_eq!(expectedrepofileobj, repofileobj);
    }

    #[test]
    fn should_parse_repo_name() {
        let lines: Vec<String> = vec![
            "repo_name=\"something\"".into(),
        ];
        let mut expectedrepofileobj = RepoFile::new();
        expectedrepofileobj.repo_name = Some("something".into());
        let repofileobj = parse_repo_file_from_lines(lines);
        assert_eq!(expectedrepofileobj, repofileobj);
    }

    #[test]
    fn should_parse_remote_branch() {
        let lines: Vec<String> = vec![
            "remote_branch=\"something\"".into(),
        ];
        let mut expectedrepofileobj = RepoFile::new();
        expectedrepofileobj.remote_branch = Some("something".into());
        let repofileobj = parse_repo_file_from_lines(lines);
        assert_eq!(expectedrepofileobj, repofileobj);
    }
}
