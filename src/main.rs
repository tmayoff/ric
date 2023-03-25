use clap::Parser;
use docker_api::opts::{ContainerCreateOpts, LogsOpts};
use futures_util::stream::StreamExt;
use std::error::Error;

mod docker;

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

fn setup_signal_handler(
    container_id: docker_api::Id,
    docker: docker_api::Docker,
) -> Result<(), ctrlc::Error> {
    let container_id = Some(container_id);
    ctrlc::set_handler(move || {
        log::info!("Stopping container");
        let mut container_id = container_id.clone();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let container = docker.containers().get(container_id.take().unwrap());
                if let Err(e) = container.kill(None).await {
                    log::error!("Failed to stop container {}", e)
                }

                docker::cleanup_container(&container).await;
            });
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();

    let current_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let current_user = format!("{}:{}", users::get_current_uid(), users::get_current_gid());

    if args.command.is_empty() {
        log::warn!("No command was specified, finishing early");
        return Ok(());
    }

    let mut docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Docker must be running");
    docker.adjust_api_version().await?;

    docker::pull_if_needed(&docker, &args.image).await?;

    let mut mounts = args.mounts.unwrap_or_default();
    mounts.push(format!("{}:/tmp", current_dir));

    let container_opts = ContainerCreateOpts::builder()
        .image(args.image)
        .volumes(mounts)
        .working_dir("/tmp")
        .command(args.command)
        .user(current_user)
        .build();

    let container = docker
        .containers()
        .create(&container_opts)
        .await
        .expect("Failed to create container");

    if let Err(e) = setup_signal_handler(container.id().clone(), docker) {
        log::error!("Failed to setup error handler exiting early ({})", e);
        return Ok(());
    }

    container.start().await?;

    log::debug!("Started container");

    let mut logs = container.logs(
        &LogsOpts::builder()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .build(),
    );
    while let Some(logs) = logs.next().await {
        let log = match logs? {
            docker_api::conn::TtyChunk::StdOut(s) => s,
            docker_api::conn::TtyChunk::StdErr(e) => e,
            docker_api::conn::TtyChunk::StdIn(e) => e,
        };

        let log = String::from_utf8_lossy(&log).to_string();
        print!("{}", log);
    }

    if let Err(e) = container.wait().await {
        log::error!("Failed to wait for container {}", e);
        docker::cleanup_container(&container).await;
    }

    docker::cleanup_container(&container).await;

    Ok(())
}
