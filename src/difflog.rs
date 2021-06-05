use std::io;
use terminal_size::{Width, terminal_size};


use super::cli::MgtCommandDifflog;
use super::topbase::find_a_b_difference;
use super::topbase::ABTraversalMode;
use crate::{git_helpers3::Commit, topbase::ConsecutiveCommitGroup};

pub fn format_right_string(commit: &Commit) -> String {
    format!("{}", commit.summary)
}

pub fn format_left_string(
    commit: &Commit,
    left_allowance: usize,
) -> String {
    let summary_len = commit.summary.len();
    if summary_len > left_allowance {
        let minus_5 = &commit.summary[0..left_allowance - 5];
        format!("{}[...]", minus_5)
    } else {
        let extra_spaces = left_allowance - summary_len;
        format!("{}{}", commit.summary, " ".repeat(extra_spaces))
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
        out = format!("{}{}", out, format_left_string(left_commit, left_allowance));

        if countdown == 0 {
            let (right_commit, _) = &right_group.commits[i - ahead_by];
            out = format!("{}{}{}", out, seperator, format_right_string(right_commit));
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
    ahead_by: usize
) -> String {
    let mut out = "".into();

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
        format_group_string_right_ahead(left_group, right_group, right_commits - left_commits)
    }
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
    let (left_uniq, right_uniq) = find_a_b_difference(
        branch_left, branch_right, cmd.traversal_mode, true)?;

    // TODO: how to nicely display full diff log?
    // its easiest to just show the top group, but what if theres
    // many groups?

    // eprintln!("{:#?}", left_uniq);
    // eprintln!("{:#?}", right_uniq);

    let default_empty = ConsecutiveCommitGroup {
        commits: vec![],
        is_shared: false,
    };
    let left_first_group = left_uniq.first().unwrap_or(&default_empty);
    let right_first_group = right_uniq.first().unwrap_or(&default_empty);

    println!("{}", format_group_string(left_first_group, right_first_group, term_width));

    Ok(())
}

pub fn run_difflog(cmd: &mut MgtCommandDifflog) {
    if let Err(e) = run_actual(cmd) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
