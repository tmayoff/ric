use assert_cmd::prelude::*;
use docker_api::opts::ContainerCreateOpts;
use futures_util::StreamExt;
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

#[test]
fn one_long_command_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args([
        "--image",
        "debian",
        "--root",
        "--",
        r#"ls | wc -l && echo "Hello World""#,
    ]);
    cmd.assert().success();

    let got_output = get_output(&cmd.output()?);
    assert!(!got_output.is_empty());

    let mut expected_cmd = Command::new("/bin/sh");
    expected_cmd.args(["-c", "ls | wc -l"]);
    expected_cmd.assert().success();
    let mut expected_output = get_output(&expected_cmd.output()?);
    expected_output.push_str("Hello World\n");

    println!("Got: {}", got_output);
    println!("Expected: {}", expected_output);

    assert!(expected_output == got_output);

    Ok(())
}

#[test]
fn root_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args(["--image", "debian", "--root", "--", "whoami"]);
    cmd.assert().success();

    let got_output = get_output(&cmd.output()?);
    assert!(!got_output.is_empty());

    let expected_output = String::from("root\n");

    println!("Got: {}", got_output);
    println!("Expected: {}", expected_output);

    assert!(expected_output == got_output);

    Ok(())
}

#[test]
fn env_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ric")?;
    cmd.env("RIC_IMAGE", "debian");
    cmd.args(["--", "cat", ".gitignore"]);
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

#[tokio::test(flavor = "multi_thread")]
async fn existing_container() -> Result<(), Box<dyn std::error::Error>> {
    let image = "debian";

    let mut docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Docker must be running");
    docker.adjust_api_version().await?;

    let images = docker.images();
    let mut stream = images.pull(&docker_api::opts::PullOpts::builder().image(image).build());

    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(output) => log::info!("{output:?}"),
            Err(e) => log::error!("{e}"),
        }
    }

    let opts = ContainerCreateOpts::builder()
        .image(image)
        .name("run_in_container")
        .command(vec!["tail", "-f", "/dev/null"])
        .auto_remove(true)
        .build();
    let container = docker.containers().create(&opts).await?;
    container.start().await?;

    let mut cmd = Command::cargo_bin("ric")?;
    cmd.args(["--container", "run_in_container", "--", "ls", "/"]);
    cmd.assert().success();

    let got_output = get_output(&cmd.output()?);
    assert!(!got_output.is_empty());

    container.kill(None).await?;

    Ok(())
}
