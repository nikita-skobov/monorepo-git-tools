use std::process::Command;
use std::{io::Error, process::Stdio, ffi::OsStr};

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

pub fn executed_successfully(exe_and_args: &[&str]) -> bool {
    match execute(exe_and_args) {
        Err(_) => false,
        Ok(cmd_output) => cmd_output.status == 0,
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
