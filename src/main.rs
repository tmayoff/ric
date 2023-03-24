use std::ffi::OsString;

use docker_api::opts::{ContainerCreateOpts, LogsOpts};
use futures_util::stream::StreamExt;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Missing commnd after --")]
    MissingCommand,
}

struct Input {
    args: Vec<String>,
    command: Vec<String>,
}

async fn parse_input() -> Result<Input, Error> {
    let mut args: Vec<String> = std::env::args_os()
        .map(|s| s.into_string().expect("Failed to decode input"))
        .collect();

    let _executable_path = args.first().expect("No executable path");
    log::debug!("Executable path: {:?}", _executable_path);

    if !args.contains(&String::from("--")) {
        return Err(Error::MissingCommand);
    }

    let command = args
        .split_off(args.iter().position(|s| s == "--").unwrap() + 1)
        .iter()
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();
    args.remove(args.len() - 1);

    log::debug!("Args {:?}", args);
    log::debug!("Command {:?}", command);

    Ok(Input { args, command })
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let input = parse_input().await;
    if let Err(e) = input {
        log::error!("{:?}", e);
        return;
    }
    let input = input.unwrap();

    let docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Needs docker container");

    let container_opts = ContainerCreateOpts::builder()
        .image("debian:bookworm")
        .volumes(["/home/tyler/src/ric/:/tmp/cwd"])
        .command(input.command)
        .build();

    let container = docker
        .containers()
        .create(&container_opts)
        .await
        .expect("Failed to create container");

    let res = container.start().await;
    match res {
        Ok(_) => println!("Started container"),
        Err(e) => println!("Failed to start container: {}", e),
    }

    let mut logs = container.logs(&LogsOpts::builder().stdout(true).stderr(true).build());
    while let Some(s) = logs.next().await {
        match s {
            Ok(s) => {
                let log = match s {
                    docker_api::conn::TtyChunk::StdOut(s) => s,
                    docker_api::conn::TtyChunk::StdErr(e) => e,
                    docker_api::conn::TtyChunk::StdIn(e) => e,
                };

                let log = String::from_utf8(log).expect("Failed to convert to utf8");

                print!("{}", log)
            }
            Err(e) => println!("Failed to get log: {}", e),
        }
    }

    container
        .wait()
        .await
        .expect("Failed to wait for container");
}
