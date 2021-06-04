use std::io;
use terminal_size::{Width, terminal_size};


use super::cli::MgtCommandDifflog;
use super::topbase::find_a_b_difference;
use super::topbase::ABTraversalMode;
use crate::{git_helpers3::Commit, topbase::ConsecutiveCommitGroup};

pub fn format_string_commit(commit: &Commit) -> String {
    format!("{}", commit.summary)
}

pub fn format_group_string_left_ahead(
    left_group: &ConsecutiveCommitGroup,
    right_group: &ConsecutiveCommitGroup,
    ahead_by: usize
) -> String {
    let mut out = "".into();

    let mut countdown = ahead_by;
    for (i, (left_commit, _)) in left_group.commits.iter().enumerate() {
        out = format!("{}{}", out, format_string_commit(left_commit));

        if countdown == 0 {
            let (right_commit, _) = &right_group.commits[i - ahead_by];
            out = format!("{}\t\t\t{}", out, format_string_commit(right_commit));
        } else {
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
    right_group: &ConsecutiveCommitGroup
) -> String {
    let left_commits = left_group.commits.len();
    let right_commits = right_group.commits.len();
    if left_commits >= right_commits {
        format_group_string_left_ahead(left_group, right_group, left_commits - right_commits)
    } else {
        format_group_string_right_ahead(left_group, right_group, right_commits - left_commits)
    }
}

pub fn run_actual(cmd: &mut MgtCommandDifflog) -> io::Result<()> {
    let branch_left = &cmd.branches[0];
    let branch_right = &cmd.branches[1];

    println!("Comparing {} vs {}", branch_left, branch_right);
    let (left_uniq, right_uniq) = find_a_b_difference(
        branch_left, branch_right, ABTraversalMode::Topbase)?;

    // TODO: how to nicely display full diff log?
    // its easiest to just show the top group, but what if theres
    // many groups?

    let default_empty = ConsecutiveCommitGroup {
        commits: vec![],
    };
    let left_first_group = left_uniq.first().unwrap_or(&default_empty);
    let right_first_group = right_uniq.first().unwrap_or(&default_empty);

    println!("{}", format_group_string(left_first_group, right_first_group));

    Ok(())
}

pub fn run_difflog(cmd: &mut MgtCommandDifflog) {
    if let Err(e) = run_actual(cmd) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
