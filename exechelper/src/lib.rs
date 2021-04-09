use std::process::Command;
use std::{io::Error, process::Stdio, ffi::OsStr};

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

// pub enun


pub fn executed_successfully(exe_and_args: &[&str]) -> bool {
    match execute(exe_and_args) {
        Err(_) => false,
        Ok(cmd_output) => cmd_output.status == 0,
    }
}

/// useful when you only care if an execution yielded an error
/// if the return is None, you know it was successful
pub fn executed_with_error(exe_and_args: &[&str]) -> Option<String> {
    match execute(exe_and_args) {
        Err(e) => Some(format!("{}", e)),
        Ok(o) => match o.status {
            0 => None,
            _ => Some(o.stderr.lines().next().unwrap().to_string()),
        },
    }
}

pub fn execute_with_env(
    exe_and_args: &[&str],
    keys: &[&str],
    vals: &[&str],
) -> Result<CommandOutput, Error> {
    // at the very least must provide the executable name
    assert!(exe_and_args.len() >= 1);
    assert!(keys.len() == vals.len());

    let mut proc = Command::new(exe_and_args[0]);
    for arg in exe_and_args.iter().skip(1) {
        proc.arg(arg);
    }

    let it = keys.iter().zip(vals.iter());
    for (k, v) in it {
        proc.env(k, v);
    }

    proc.stdin(Stdio::null());
    let output = proc.output();

    match output {
        Err(e) => Err(e),
        Ok(out) => {
            let errput_str_cow = String::from_utf8_lossy(&out.stderr);
            let output_str_cow = String::from_utf8_lossy(&out.stdout);
            Ok(
                CommandOutput {
                    stdout: output_str_cow.into_owned(),
                    stderr: errput_str_cow.into_owned(),
                    status: out.status.code().unwrap_or(1),
                }
            )
        }
    }
} 

pub fn execute(exe_and_args: &[&str]) -> Result<CommandOutput, Error> {
    execute_with_env(exe_and_args, &[], &[])
}

/// optionally pass in what kind of stdio config
/// you want to use for each stream. passing None
/// will use whatever the default is.
pub fn spawn_with_env_ex(
    exe_and_args: &[&str],
    keys: &[&str],
    vals: &[&str],
    read_stdin: Option<Stdio>,
    read_stderr: Option<Stdio>,
    read_stdout: Option<Stdio>,
) -> Result<std::process::Child, Error> {
    // at the very least must provide the executable name
    assert!(exe_and_args.len() >= 1);
    assert!(keys.len() == vals.len());

    let mut proc = Command::new(exe_and_args[0]);
    for arg in exe_and_args.iter().skip(1) {
        proc.arg(arg);
    }

    let it = keys.iter().zip(vals.iter());
    for (k, v) in it {
        proc.env(k, v);
    }

    let mut read_stdin = read_stdin;
    let mut read_stderr = read_stderr;
    let mut read_stdout = read_stdout;
    if let Some(cfg) = read_stdin.take() {
        proc.stdin(cfg);
    }
    if let Some(cfg) = read_stderr.take() {
        proc.stderr(cfg);
    }
    if let Some(cfg) = read_stdout.take() {
        proc.stdout(cfg);
    }

    proc.spawn()
}

pub fn spawn(exe_and_args: &[&str]) -> Result<std::process::Child, Error> {
    spawn_with_env(exe_and_args, &[], &[])
}

pub fn spawn_with_env(
    exe_and_args: &[&str],
    keys: &[&str],
    vals: &[&str],
) -> Result<std::process::Child, Error> {
    spawn_with_env_ex(exe_and_args, keys, vals,
        Some(Stdio::null()), Some(Stdio::null()), None)
}
