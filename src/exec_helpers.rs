use std::process::Command;
use std::process::Stdio;

pub fn executed_successfully(exe_and_args: &[&str]) -> bool {
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
        Err(_) => false,
        Ok(out) => out.status.success()
    }
}
