use std::process::Command;
use std::process::Stdio;

pub fn get_latest_commit() -> String {
    let command_and_args = [
        "git", "log", "--oneline",
        "--pretty=%h", "-n", "1",
    ];
    let mut proc = Command::new(command_and_args[0]);
    proc.args(&command_and_args[1..]);
    
    proc.stdin(Stdio::null());
    let output = proc.output();


    let err_str = "Haha oops! this build failed to get the latest commit hash. ¯\\_(ツ)_/¯";
    match output {
        Err(_) => return err_str.into(),
        Ok(out) => {
            let status_code = out.status.code().unwrap_or(1);
            if status_code == 0 {
                return String::from_utf8_lossy(&out.stdout).into();
            } else {
                return err_str.into();
            }
        }
    }
} 

fn main() {
    let latest_commit = get_latest_commit();
    println!("cargo:rustc-env=LATEST_COMMIT={}", latest_commit);
}
