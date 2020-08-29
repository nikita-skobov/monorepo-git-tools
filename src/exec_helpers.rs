use std::process::Command;
use std::{io::Error, process::Stdio};

pub struct CommandOutput {
    stdout: String,
    status: i32,
}

pub fn executed_successfully(exe_and_args: &[&str]) -> bool {
    match execute(exe_and_args) {
        Err(_) => false,
        Ok(cmd_output) => cmd_output.status == 0,
    }
}

pub fn execute(exe_and_args: &[&str]) -> Result<CommandOutput, Error> {
    // at the very least must provide the executable name
    assert!(exe_and_args.len() >= 1);

    let mut proc = Command::new(exe_and_args[0]);
    for arg in exe_and_args.iter().skip(1) {
        proc.arg(arg);
    }
    proc.stdin(Stdio::null());
    proc.stderr(Stdio::null());
    let output = proc.output();

    match output {
        Err(e) => Err(e),
        Ok(out) => {
            let output_str_cow = String::from_utf8_lossy(&out.stdout);
            Ok(
                CommandOutput {
                    stdout: output_str_cow.into_owned(),
                    status: out.status.code().unwrap_or(1),
                }
            )
        }
    }
}
