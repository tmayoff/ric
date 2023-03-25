use anyhow::bail;
use clap::Parser;

mod docker;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(short, long, env = "RIC_IMAGE")]
    image: Option<String>,

    #[arg(short, long, env = "RIC_CONTAINER")]
    container: Option<String>,

    #[arg(short, long, env = "RIC_MOUNTS")]
    mounts: Option<Vec<String>>,

    #[arg(short, long)]
    root: bool,

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
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let args = Args::parse();

    if args.command.is_empty() {
        log::warn!("No command was specified, finishing early");
        return Ok(());
    }

    if args.image.is_none() && args.container.is_none() {
        bail!("Must provide either an image or a container to use");
    }

    let mut docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Docker must be running");
    docker.adjust_api_version().await?;

    if let Some(image) = &args.image {
        docker::pull_if_needed(&docker, image).await?;
    }

    docker::runner(&docker, args).await?;

    Ok(())
}
