use anyhow::bail;
use futures_util::stream::StreamExt;

use docker_api::{
    opts::{
        ContainerCreateOpts, ContainerFilter, ContainerListOpts, ExecCreateOpts, LogsOpts, PullOpts,
    },
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

async fn start_container(
    docker: &docker_api::Docker,
    image: &str,
    command: Vec<String>,
    mounts: Vec<String>,
    user: &str,
) -> Result<docker_api::Container, anyhow::Error> {
    let container_opts = ContainerCreateOpts::builder()
        .image(image)
        .volumes(mounts)
        .working_dir("/tmp")
        .user(user)
        .command(command);

    Ok(docker
        .containers()
        .create(&container_opts.build())
        .await
        .expect("Failed to create container"))
}

pub async fn runner(docker: &docker_api::Docker, args: Args) -> Result<(), anyhow::Error> {
    let command = args.command.clone();

    let mut mounts = args.mounts.clone().unwrap_or_default();
    let current_dir = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let user = if args.root {
        String::from("0:0")
    } else {
        format!("{}:{}", users::get_current_uid(), users::get_current_gid())
    };

    mounts.push(format!("{}:/tmp", current_dir));

    if let Some(image) = args.image {
        let container = start_container(docker, &image, command, mounts, &user).await?;

        setup_signal_handler(container.id().clone(), docker.clone())?;

        container.start().await?;

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
            cleanup_container(&container).await;
        }

        cleanup_container(&container).await;
    } else if let Some(container_name) = args.container {
        let c = docker
            .containers()
            .list(
                &ContainerListOpts::builder()
                    .filter(vec![ContainerFilter::Name(container_name.clone())])
                    .build(),
            )
            .await?;

        let container = docker
            .containers()
            .get(c.first().unwrap().id.clone().unwrap());

        let mut logs = container.exec(
            &ExecCreateOpts::builder()
                .command(args.command)
                .user(user)
                .attach_stdout(true)
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
    } else {
        bail!("No Image or Container specified");
    }

    Ok(())
}
