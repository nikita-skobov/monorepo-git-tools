use std::io;
use terminal_size::{Width, terminal_size};


use super::cli::MgtCommandDifflog;
use super::topbase::find_a_b_difference2;
use super::git_helpers3::Commit;

pub fn format_right_string(
    commit: &str,
    right_allowance: usize,
) -> String {
    let summary_len = commit.len();
    if summary_len > right_allowance {
        let minus_5 = &commit[0..right_allowance - 5];
        format!("{}[...]", minus_5)
    } else {
        format!("{}", commit)
    }
}

pub fn format_left_string(
    commit: &str,
    left_allowance: usize,
) -> String {
    let summary_len = commit.len();
    if summary_len > left_allowance {
        let minus_5 = &commit[0..left_allowance - 5];
        format!("{}[...]", minus_5)
    } else {
        let extra_spaces = left_allowance - summary_len;
        format!("{}{}", commit, " ".repeat(extra_spaces))
    }
}

pub fn format_group_string_left_ahead(
    left_group: &Vec<Commit>,
    right_group: &Vec<Commit>,
    ahead_by: usize,
    term_width: usize,
) -> String {
    let mut out = "".into();

    let approx_half = term_width / 2;
    let seperator = " | ";
    let left_allowance = approx_half - seperator.len();
    // let right_allowance = approx_half;

    let mut countdown = ahead_by;
    for (i, left_commit) in left_group.iter().enumerate() {
        out = format!("{}{}", out, format_left_string(&left_commit.summary, left_allowance));

        if countdown == 0 {
            let right_commit = &right_group[i - ahead_by];
            out = format!("{}{}{}", out, seperator, format_right_string(&right_commit.summary, left_allowance));
        } else {
            out = format!("{}{}", out, seperator);
            countdown -= 1;
        }
        out.push('\n');
    }

    out
}

pub fn format_group_string_right_ahead(
    left_group: &Vec<Commit>,
    right_group: &Vec<Commit>,
    ahead_by: usize,
    term_width: usize,
) -> String {
    let mut out = "".into();

    let approx_half = term_width / 2;
    let seperator = " | ";
    let left_allowance = approx_half - seperator.len();
    let empty_left_string = " ".repeat(left_allowance);

    let mut countdown = ahead_by;
    for (i, right_commit) in right_group.iter().enumerate() {
        if countdown != 0 {
            // add the empty left string first
            out = format!("{}{}", out, empty_left_string);
            countdown -= 1;
        } else {
            let left_commit = &left_group[i - ahead_by];
            out = format!("{}{}", out, format_left_string(&left_commit.summary, left_allowance));
        }

        out = format!("{}{}{}\n", out, seperator, format_right_string(&right_commit.summary, left_allowance));
    }

    out
}

pub fn format_group_string(
    left_group: &Vec<Commit>,
    right_group: &Vec<Commit>,
    term_width: usize,
) -> String {
    let left_commits = left_group.len();
    let right_commits = right_group.len();
    if left_commits >= right_commits {
        format_group_string_left_ahead(left_group, right_group, left_commits - right_commits, term_width)
    } else {
        format_group_string_right_ahead(left_group, right_group, right_commits - left_commits, term_width)
    }
}

/// should be guaranteed that there is at
/// least one commit in each group
pub fn format_fork_point(
    left_group: &Vec<Commit>,
    right_group: &Vec<Commit>,
    term_width: usize
) -> String {
    let (left_hash, left_summary) = match left_group.first() {
        Some(c) => (c.id.short(), c.summary.clone()),
        None => ("???????", "".to_string()),
    };
    let (right_hash, right_summary) = match right_group.first() {
        Some(c) => (c.id.short(), c.summary.clone()),
        None => ("???????", "".to_string()),
    };

    let approx_half = term_width / 2;
    let seperator = " <===> ";
    // + 2 so it lines up with the | character in the other formatting
    let left_allowance = approx_half - seperator.len() + 2;
    // minus 1 because we will put 1 space
    let left_str = format_left_string(&left_summary, left_allowance - left_hash.len() - 1);
    let left_str = format!("{} {}", left_str, left_hash);
    let right_str = format_right_string(&right_summary, left_allowance - right_hash.len() - 1);
    let right_str = format!("{} {}", right_hash, right_str);
    format!("{}{}{}\n", left_str, seperator, right_str)
}

pub fn format_title(
    left_ref_name: &str,
    right_ref_name: &str,
    term_width: usize
) -> String {
    let approx_half = term_width / 2;
    let seperator = " | ";
    let left_allowance = approx_half - seperator.len();

    let bottom_seperator = "=".repeat(term_width);
    let left_str = format_left_string(left_ref_name, left_allowance);
    let right_str = right_ref_name;
    format!("{}{}{}\n{}", left_str,seperator, right_str, bottom_seperator)
}

pub fn run_actual(cmd: &mut MgtCommandDifflog) -> io::Result<()> {
    let branch_left = &cmd.branches[0];
    let branch_right = &cmd.branches[1];

    let term_width = if let Some(w) = cmd.term_width {
        w
    } else {
        if let Some((Width(w), _)) = terminal_size() {
            w as usize
        } else {
            // just guess 120 idk
            120
        }
    };

    // TODO: make this a cli option
    let traverse_at_a_time = 500;
    let topbase_res = find_a_b_difference2(
        branch_left, branch_right, Some(traverse_at_a_time), true)?;
    let successful_topbase = match topbase_res {
        Some(s) => s,
        None => {
            println!("Failed to find a fork point");
            return Ok(());
        }
    };

    let left_group = successful_topbase.top_commits;
    let right_group = vec![];

    let left_fork = vec![successful_topbase.fork_point.0];
    let right_fork = vec![successful_topbase.fork_point.1];
    println!("{}", format_title(branch_left, branch_right, term_width));
    print!("{}", format_group_string(&left_group, &right_group, term_width));
    println!("{}", format_fork_point(&left_fork, &right_fork, term_width));

    Ok(())
}

pub fn run_difflog(cmd: &mut MgtCommandDifflog) {
    if let Err(e) = run_actual(cmd) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
