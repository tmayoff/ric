use assert_cmd::prelude::*;
use std::process::{Command, Output};

fn get_output(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn ls_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args(["--image", "debian", "--", "ls"]);
    cmd.assert().success();

    let mut expected_cmd = Command::new("ls");
    expected_cmd.assert().success();

    let got_output = get_output(&cmd.output()?);
    assert!(!got_output.is_empty());
    let expected_output = get_output(&expected_cmd.output()?);
    assert!(!expected_output.is_empty());

    println!("Got: {}", got_output);
    println!("Expected: {}", expected_output);

    assert!(expected_output == got_output);

    Ok(())
}

#[test]
fn cat_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args(["--image", "debian", "--", "cat", ".gitignore"]);
    cmd.assert().success();

    let mut expected_cmd = Command::new("cat");
    expected_cmd.arg(".gitignore");
    expected_cmd.assert().success();

    let got_output = get_output(&cmd.output()?);
    assert!(!got_output.is_empty());
    let expected_output = get_output(&expected_cmd.output()?);
    assert!(!expected_output.is_empty());

    println!("Got: {}", got_output);
    println!("Expected: {}", expected_output);

    assert!(expected_output == got_output);

    Ok(())
}
