use futures_util::stream::StreamExt;

use docker_api::{
    opts::{ContainerCreateOpts, PullOpts},
    Docker,
};

use crate::{setup_signal_handler, Args};

fn append_tag(image: &str) -> String {
    if image.contains(':') {
        image.to_string()
    } else {
        format!("{}:latest", image)
    }
}

pub async fn cleanup_container(container: &docker_api::Container) {
    let res = container.delete().await;
    if let Err(e) = res {
        log::error!("Error removing container {}", e);
    }
}

pub async fn pull_if_needed(docker: &Docker, image: &str) -> Result<(), anyhow::Error> {
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

pub async fn runner(
    docker: &docker_api::Docker,
    args: Args,
) -> Result<docker_api::Container, anyhow::Error> {
    let current_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let current_user = format!("{}:{}", users::get_current_uid(), users::get_current_gid());

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

    setup_signal_handler(container.id().clone(), docker.clone())?;

    container.start().await?;

    Ok(container)
}
