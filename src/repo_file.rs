use std::path::Path;
use std::process;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug)]
pub struct RepoFile {
    remote_repo: String,
    include_as: Vec<String>,
}

#[derive(PartialEq)]
enum RepoFileVariableType {
    TypeString,
    TypeArray,
    TypeUnknown,
}
use RepoFileVariableType::*;

struct RepoFileVariable {
    name: String,
    value: Vec<String>,
    complete: bool,
    var_type: RepoFileVariableType,
}

const EMPTY_STRING: &str = "";

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

fn parse_variable(variable: &mut RepoFileVariable, text: String) {
    if variable.name == EMPTY_STRING && text.contains("=") {
        // variable is empty, and this line
        // contains an equal sign, so we assume the variable
        // is being created
        variable.name = get_variable_name(&text);
        variable.var_type = match char_after_equals(&text) {
            '(' => TypeArray,
            '"' => TypeString,
             _  => TypeUnknown,
        };
    }

    // easiest case to parse. just get everything between the quotes
    if variable.name != EMPTY_STRING && variable.var_type == TypeString {
        let strings = get_all_strings(&text);
        if let None = strings {
            println!("Failed to parse variable at line:\n{}", text);
            process::exit(1);
        }

        // there should only be one string
        variable.value = vec![strings.unwrap()[0].clone()];
        variable.complete = true;
    }
    if variable.name != EMPTY_STRING && variable.var_type == TypeArray {
        let strings = get_all_strings(&text);
        if let None = strings {
            println!("Failed to parse variable at line:\n{}", text);
            process::exit(1);
        }

        // add all of the strings we found on this line
        variable.value.append(&mut strings.unwrap());
        variable.complete = is_end_of_array(&text);
    }
}

fn add_variable_to_repo_file(repofile: &mut RepoFile, variable: &mut RepoFileVariable) {
    if variable.var_type == TypeArray {
        repofile.include_as = variable.value.clone();
    }
    if variable.var_type == TypeString {
        repofile.remote_repo = variable.value[0].clone();
    }

    variable.name = EMPTY_STRING.to_string();
    variable.value = vec![EMPTY_STRING.to_string()];
}


fn not_a_full_line_comment(text: &String) -> bool {
    let mut is_full_line_comment = false;
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }

        if c == '#' {
            is_full_line_comment = true;
        }
        break;
    }

    return !is_full_line_comment;
}

pub fn parse_repo_file(filename: &str) -> RepoFile {
    let repo_file_path = Path::new(filename);
    if !repo_file_path.exists() {
        println!("Failed to find repo_file: {}", filename);
        process::exit(1);
    }

    let file = File::open(repo_file_path);
    if let Err(file_error) = file {
        println!("Failed to open file: {}, {}", filename, file_error);
        process::exit(1);
    }

    let mut repofile_obj = RepoFile {
        remote_repo: EMPTY_STRING.to_string(),
        include_as: vec![EMPTY_STRING.to_string()],
    };

    // this will be modified by the parse_variable func above
    // everytime this variable is "complete", it will be added
    // to the RepoFile struct
    let mut current_variable = RepoFileVariable{
        name: EMPTY_STRING.to_string(),
        value: vec![EMPTY_STRING.to_string()],
        complete: false,
        var_type: TypeUnknown,
    };

    let file_contents = file.unwrap();
    let reader = BufReader::new(file_contents);
    for (_, line_res) in reader.lines().enumerate() {
        let line = line_res.unwrap();
        println!("line: {}", line);
        if not_a_full_line_comment(&line) {
            parse_variable(&mut current_variable, line);
        }

        if current_variable.complete {
            add_variable_to_repo_file(&mut repofile_obj, &mut current_variable);
            current_variable.complete = false;
        }
    }

    println!("repo file obj: {:?}", repofile_obj);
    return repofile_obj;
}
