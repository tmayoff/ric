use clap::Parser;
use docker_api::{
    opts::{ContainerCreateOpts, LogsOpts, PullOpts},
    Docker,
};
use futures_util::stream::StreamExt;
use std::error::Error;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    image: String,

    #[arg(short, long)]
    container: Option<String>,

    #[arg(short, long)]
    mounts: Option<Vec<String>>,

    #[arg(last = true)]
    command: Vec<String>,
}

fn append_tag(image: &str) -> String {
    if image.contains(':') {
        image.to_string()
    } else {
        format!("{}:latest", image)
    }
}

async fn pull_if_needed(docker: &Docker, image: &str) -> Result<(), Box<dyn Error>> {
    let images = docker.images();

    for i in images.list(&Default::default()).await?.into_iter() {
        let image = append_tag(image);
        if i.repo_tags.contains(&image) {
            log::debug!("Image already downloaded");
            return Ok(());
        }
    }

    log::debug!("Pulling image");
    let mut stream = images.pull(&PullOpts::builder().image(image).build());

    while let Some(pull_result) = stream.next().await {
        match pull_result {
            Ok(output) => log::info!("{output:?}"),
            Err(e) => log::error!("{e}"),
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();
    let current_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    if args.command.is_empty() {
        log::warn!("Command is empty, finishing early");
        return Ok(());
    }

    let mut docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Docker must be running");
    docker.adjust_api_version().await?;

    pull_if_needed(&docker, &args.image).await?;

    let mut mounts = args.mounts.unwrap_or_default();
    mounts.push(format!("{}:/tmp", current_dir));

    log::debug!("{:?}", mounts);

    let container_opts = ContainerCreateOpts::builder()
        .image(args.image)
        .volumes(mounts)
        .command(args.command)
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


    container.remove(&Default::default()).await?;

    Ok(())
}
