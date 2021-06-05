use std::io;
use terminal_size::{Width, terminal_size};


use super::cli::MgtCommandDifflog;
use super::topbase::find_a_b_difference;
use crate::topbase::ConsecutiveCommitGroup;

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
    left_group: &ConsecutiveCommitGroup,
    right_group: &ConsecutiveCommitGroup,
    ahead_by: usize,
    term_width: usize,
) -> String {
    let mut out = "".into();

    let approx_half = term_width / 2;
    let seperator = " | ";
    let left_allowance = approx_half - seperator.len();
    // let right_allowance = approx_half;

    let mut countdown = ahead_by;
    for (i, (left_commit, _)) in left_group.commits.iter().enumerate() {
        out = format!("{}{}", out, format_left_string(&left_commit.summary, left_allowance));

        if countdown == 0 {
            let (right_commit, _) = &right_group.commits[i - ahead_by];
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
    left_group: &ConsecutiveCommitGroup,
    right_group: &ConsecutiveCommitGroup,
    ahead_by: usize,
    term_width: usize,
) -> String {
    let mut out = "".into();

    let approx_half = term_width / 2;
    let seperator = " | ";
    let left_allowance = approx_half - seperator.len();
    let empty_left_string = " ".repeat(left_allowance);

    let mut countdown = ahead_by;
    for (i, (right_commit, _)) in right_group.commits.iter().enumerate() {
        if countdown != 0 {
            // add the empty left string first
            out = format!("{}{}", out, empty_left_string);
            countdown -= 1;
        } else {
            let (left_commit, _) = &left_group.commits[i - ahead_by];
            out = format!("{}{}", out, format_left_string(&left_commit.summary, left_allowance));
        }

        out = format!("{}{}{}\n", out, seperator, format_right_string(&right_commit.summary, left_allowance));
    }

    out
}

pub fn format_group_string(
    left_group: &ConsecutiveCommitGroup,
    right_group: &ConsecutiveCommitGroup,
    term_width: usize,
) -> String {
    let left_commits = left_group.commits.len();
    let right_commits = right_group.commits.len();
    if left_commits >= right_commits {
        format_group_string_left_ahead(left_group, right_group, left_commits - right_commits, term_width)
    } else {
        format_group_string_right_ahead(left_group, right_group, right_commits - left_commits, term_width)
    }
}

/// should be guaranteed that there is at
/// least one commit in each group
pub fn format_fork_point(
    left_group: &ConsecutiveCommitGroup,
    right_group: &ConsecutiveCommitGroup,
    term_width: usize
) -> String {
    let (left_hash, left_summary) = match left_group.commits.first() {
        Some((c, _)) => (c.id.short(), c.summary.clone()),
        None => ("???????", "".to_string()),
    };
    let (right_hash, right_summary) = match right_group.commits.first() {
        Some((c, _)) => (c.id.short(), c.summary.clone()),
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

    println!("Comparing {} vs {}", branch_left, branch_right);
    let (left_list, right_list) = find_a_b_difference(
        branch_left, branch_right, cmd.traversal_mode, true)?;

    let default_empty = ConsecutiveCommitGroup {
        commits: vec![],
        is_shared: false,
    };

    let left_len = left_list.len();
    let right_len = right_list.len();
    let longest = if left_len >= right_len { left_len } else { right_len };
    let mut left_list_iter = left_list.iter();
    let mut right_list_iter = right_list.iter();
    for _ in 0..longest {
        let left_group = left_list_iter.next().unwrap_or(&default_empty);
        let right_group = right_list_iter.next().unwrap_or(&default_empty);
        let left_is_fork = left_group.is_shared;
        let right_is_fork = right_group.is_shared;
        if left_is_fork && right_is_fork {
            // print a fork point differently:
            println!("{}", format_fork_point(left_group, right_group, term_width));
        } else {
            println!("{}", format_group_string(left_group, right_group, term_width));
        }
    }

    Ok(())
}

pub fn run_difflog(cmd: &mut MgtCommandDifflog) {
    if let Err(e) = run_actual(cmd) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
