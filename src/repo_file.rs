use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq)]
pub struct RepoFile {
    pub repo_name: Option<String>,
    pub remote_repo: Option<String>,
    pub remote_branch: Option<String>,
    pub include_as: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
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
