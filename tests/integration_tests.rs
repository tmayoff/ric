use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn simple_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args(["--", "ls", "/tmp/cwd/"]);

    let output = cmd.output().expect("Failed to get output");
    let stderr = &output.stderr;
    let stdout = &output.stdout;
    cmd.assert().success();

    println!("stderr: {}", String::from_utf8_lossy(stderr));
    println!("stdout: {}", String::from_utf8_lossy(stdout));

    Ok(())
}
