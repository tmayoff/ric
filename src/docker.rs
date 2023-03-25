use futures_util::stream::StreamExt;
use std::error::Error;

use docker_api::{opts::PullOpts, Docker};

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

pub async fn pull_if_needed(docker: &Docker, image: &str) -> Result<(), Box<dyn Error>> {
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
