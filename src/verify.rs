use super::cli::MgtCommandVerify;
use super::die;
use super::repo_file;
use super::git_helpers3;
use std::io::{self, BufRead};
use gitfilter::filter::FilterRules;
use gitfilter::filter::FilterRule;

#[derive(Debug)]
pub enum FileOpType<'a> {
    IncludeAs(&'a str, &'a str),
    Include(&'a str),
    Exclude(&'a str),
}

/// iterate over the repo file include, include_as, and
/// exclude. create a vec of file ops from that without sorting
pub fn get_vec_of_file_ops_with_order<'a>(
    repo_file: &'a repo_file::RepoFile,
    src_to_dest: bool,
) -> Vec<FileOpType<'a>> {
    let mut out_vec = vec![];
    if let Some(ref include_as) = repo_file.include_as {
        for (i, _path) in include_as.iter().enumerate() {
            if i % 2 != 0 {
                if src_to_dest {
                    out_vec.push(FileOpType::IncludeAs(&include_as[i - 1], &include_as[i]));
                } else {
                    out_vec.push(FileOpType::IncludeAs(&include_as[i], &include_as[i - 1]));
                }
            }
        }
    }
    if let Some(ref include) = repo_file.include {
        for path in include {
            out_vec.push(FileOpType::Include(path));
        }
    }
    if let Some(ref exclude) = repo_file.exclude {
        for path in exclude {
            out_vec.push(FileOpType::Exclude(path));
        }
    }
    
    out_vec
}

pub fn get_vec_of_file_ops<'a>(repo_file: &'a repo_file::RepoFile) -> Vec<FileOpType<'a>> {
    get_vec_of_file_ops_with_order(repo_file, true)
}

/// given a vec of fileops, sort it by the src/ path.
/// this means if your vec has paths like:
/// src/a/
/// src/
/// then src/ will come before src/a/
/// this is useful to establish a correct order of operations
/// when filtering
pub fn sort_vec_of_file_ops<'a>(file_ops: &mut Vec<FileOpType<'a>>) {
    file_ops.sort_by(|a, b| {
        match a {
            FileOpType::IncludeAs(src_a, _) |
            FileOpType::Include(src_a) |
            FileOpType::Exclude(src_a) => match b {
                FileOpType::IncludeAs(src_b, _) |
                FileOpType::Include(src_b) |
                FileOpType::Exclude(src_b) => {
                    src_a.cmp(src_b)
                }
            }
        }
    });
}

/// this will first sort your file_ops for you,
/// and then create the FilterRules to pass to
/// gitfilter to do the actual filtering
pub fn make_filter_rules<'a>(
    file_ops: &mut Vec<FileOpType<'a>>
) -> FilterRules {
    sort_vec_of_file_ops(file_ops);
    // // TODO: need to handle grouping by largest consecutive path?
    // // originally i thought i needed to group by largest common path
    // // and then on each group: order the include/exclude/include-as
    // // but now that I think about it, gitfilter should probably handle
    // // ordering correctly so long as the filter rules are given in
    // // lexographic order which our sort_vec_of_file_ops does above.
    // // i propose this:
    // // have 2 modes of operation for now:
    // // default is sort lexographically, and pass as-is to gitfilter
    // // second is an optional setting that says: dont parse, and
    // // instead let the user explicitly pick their order that they want
    // // and finally, if there later is a need for mgt
    // // to 'smartly figure out the correct filter order' then re-implement this
    // // and add logic to sort the filter ops within each group in the map:
    // let mut map: HashMap<&'a str, Vec<&mut FileOpType<'a>>> = HashMap::new();
    // let mut previous_key: &str = "";
    // for op in file_ops {
    //     let this_key = match op {
    //         FileOpType::IncludeAs(src, _) |
    //         FileOpType::Include(src) |
    //         FileOpType::Exclude(src) => &src[..],
    //     };
    //     if this_key.starts_with(previous_key) {
    //         match map.get_mut(previous_key) {
    //             Some(existing_vec) => { existing_vec.push(op); },
    //             None => { map.insert(previous_key, vec![op]); },
    //         }
    //     } else {
    //         map.insert(this_key, vec![op]);
    //         previous_key = this_key;
    //     }
    // }

    file_ops.drain(..).map(|fileop| {
        match fileop {
            FileOpType::IncludeAs(src, dest) => {
                // previously the requirement for include_as for renaming something to root/
                // was the second component needed to be an empty space
                // this is unnecessary for gitfilter, so we look for that, and clean it up here
                FilterRule::FilterRulePathRename(src.trim_start().into(), dest.trim_start().into())
            }
            FileOpType::Include(src) => {
                FilterRule::FilterRulePathInclude(src.into())
            }
            FileOpType::Exclude(src) => {
                FilterRule::FilterRulePathExclude(src.into())
            }
        }
    }).collect()
}

/// need to form input that gitfilter expects
/// most of it is dummy data because we are only using
/// the part of gitfilter where we decide whether or not
/// to keep a file/what to rename it.
pub fn apply_expected_gitfilter<'a>(
    all_local_files: &'a Vec<String>,
    filter_rules: &FilterRules,
) -> Vec<(usize, String)> {
    let mut filtered = vec![];
    for (i, path) in all_local_files.iter().enumerate() {
        let mut new_path = path.clone();
        if gitfilter::filter::should_use_file(&mut new_path, filter_rules, false) {
            filtered.push((i, new_path));
        }
    }

    filtered
}

pub fn run_verify(
    cmd: &mut MgtCommandVerify,
) {
    let repo_file_path = if cmd.repo_file.len() < 1 {
        die!("Must provide repo path argument");
    } else {
        cmd.repo_file[0].clone()
    };
    let repo_file = repo_file::parse_repo_file_from_toml_path(&repo_file_path);
    let mut file_ops = get_vec_of_file_ops(&repo_file);
    let filter_rules = make_filter_rules(&mut file_ops);
    let all_files: Vec<String> = if cmd.stdin {
        let stdin = io::stdin();
        let mut out = vec![];
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(o) => o,
                Err(e) => die!("Failed to read stdin: {}", e)
            };
            out.push(line);
        }
        out
    } else {
        match git_helpers3::get_all_files_in_repo() {
            Ok(text) => {
                text.split('\n').map(|line| line.to_string()).collect()
            }
            Err(e) => {
                die!("Failed to get all files in git repo:\n{}", e);
            }
        }
    };

    // eprintln!("ALL FILES: {:?}", all_files);
    let remaining_files = apply_expected_gitfilter(&all_files, &filter_rules);

    // if pretty, we imply verbose, but we print verbosely in a pretty way
    // by calculating the column size:
    let label_src = "Source";
    let label_dest = "Dest";
    if cmd.pretty {
        let col_margin_left = 1;
        let col_margin_right = col_margin_left;
        let col_char = '|';
        let mut max_left_column = 0;
        for (original_index, _file) in &remaining_files {
            let original_file_len = all_files[*original_index].len();
            if original_file_len > max_left_column {
                max_left_column = original_file_len;
            }
        }
        print!("\n{}", label_src);
        for _ in label_src.len()..max_left_column+col_margin_left {
            print!(" ");
        }
        print!("{}", col_char);
        for _ in 0..col_margin_right {
            print!(" ");
        }
        println!("{}", label_dest);
        // + 10 because i dont want to calculate the width of the right
        // column so i figure 10 is good enough, whatever
        for _ in 0..max_left_column+col_margin_left+col_margin_right+10 {
            print!("=");
        }
        println!();

        for (original_index, file) in remaining_files {
            let original_file_len = all_files[original_index].len();
            print!("{}", all_files[original_index]);
            for _ in original_file_len..max_left_column+col_margin_left {
                print!(" ");
            }
            print!("{}", col_char);
            for _ in 0..col_margin_right {
                print!(" ");
            }
            println!("{}", file);
        }

        return;
    }

    println!("\n{} -> {}", label_src, label_dest);
    for (original_index, file) in remaining_files {
        if cmd.verbose {
            println!("{} -> {}", all_files[original_index], file);
        } else {
            println!("{}", file);
        }
    }
}
