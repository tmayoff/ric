use std::ffi::OsString;

#[tokio::main]
async fn main() {
    let mut args: Vec<OsString> = std::env::args_os().collect();

    let _executable_path = args.first().expect("No executable path");
    println!("Executable path: {:?}", _executable_path);

    if !args.contains(&OsString::from("--")) {
        println!("No -- found");
        return;
    }

    let command = args.split_off(args.iter().position(|s| s == "--").unwrap() + 1);
    args.remove(args.len() - 1);

    println!("Args {:?}", args);
    println!("Command {:?}", command);

    let docker =
        docker_api::Docker::new("unix:///var/run/docker.sock").expect("Needs docker container");

    let containers = docker
        .containers()
        .list(&Default::default())
        .await
        .expect("Needs some containers");

    containers.iter().for_each(|c| {
        println!("Container: {:?}", c.names);
    });
}
