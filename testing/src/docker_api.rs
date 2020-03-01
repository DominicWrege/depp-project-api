use crate::crash_test::Error;
use crate::script::{ScriptOutput, TIMEOUT};
use bollard::container::{
    CreateContainerOptions, CreateContainerResults, HostConfig, LogOutput, LogsOptions, MountPoint,
    StartContainerOptions, WaitContainerOptions,
};
use futures::StreamExt;
use grpc_api::{Script, TargetOs};
use std::fmt::Write;

#[derive(Debug)]
pub struct Mount<'a> {
    pub source_dir: &'a str,
    pub target_dir: &'a str,
}

enum MountPermission {
    Read,
    Write,
}

impl From<MountPermission> for Option<bool> {
    fn from(m: MountPermission) -> Option<bool> {
        let p = match m {
            MountPermission::Read => true,
            MountPermission::Write => false,
        };
        Some(p)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ImageOpt {
    pub name: &'static str,
    pub platform: &'static str,
}

pub fn docker_image(script: &Script) -> ImageOpt {
    match script.target_os() {
        TargetOs::Windows => MS_IMAGE,
        TargetOs::Unix => LINUX_IMAGE,
    }
}

pub fn docker_mount_points(script: &Script) -> (&'static str, &'static str) {
    match script.target_os() {
        TargetOs::Windows => (r"C:\testing\", r"C:\script\"),
        TargetOs::Unix => ("/testing/", "/script/"),
    }
}

fn create_mount_point<'a>(
    source: &'a str,
    target: &'a str,
    permission: MountPermission,
) -> MountPoint<&'a str> {
    MountPoint {
        target,
        source,
        type_: "bind",
        read_only: permission.into(),
        consistency: "default",
        ..Default::default()
    }
}

pub const LINUX_IMAGE: ImageOpt = ImageOpt {
    name: "dominicwrege/depp-project-ubuntu:latest",
    platform: "linux",
};

pub const MS_IMAGE: ImageOpt = ImageOpt {
    name: "mcr.microsoft.com/powershell:latest",
    platform: "windows",
};

pub async fn pull_image(image_opt: ImageOpt, docker: &bollard::Docker) {
    use bollard::image::CreateImageOptions;

    let options = Some(CreateImageOptions {
        from_image: image_opt.name,
        platform: image_opt.platform,
        ..Default::default()
    });

    let mut stream = docker.create_image(options, None);
    log::info!("pulling {}", &image_opt.name);
    while let Some(s) = stream.next().await {
        if let Err(err) = s {
            log::error!(
                "Could pull image: {}. Maybe because it was not found.",
                &image_opt.name
            );
            panic!("{}", err);
        }
    }
}

pub fn create_host_config<'a>(
    out_put_mount: &'a Mount,
    script_mount: &'a Mount,
) -> Option<HostConfig<&'a str>> {
    let output_mount_point = create_mount_point(
        out_put_mount.source_dir,
        out_put_mount.target_dir,
        MountPermission::Write,
    );
    let script_mount_point = create_mount_point(
        script_mount.source_dir,
        script_mount.target_dir,
        MountPermission::Read,
    );
    //dbg!(&script_mount_point);
    //dbg!(&output_mount_point);
    Some(HostConfig {
        mounts: Some(vec![script_mount_point, output_mount_point]),
        memory: Some(256000000), //256MB RAM for each container
        ..Default::default()
    })
}

pub async fn create_container(
    cmd: Vec<&str>,
    image: &str,
    host_config: Option<HostConfig<&str>>,
    working_dir: &str,
    docker: &bollard::Docker,
) -> Result<CreateContainerResults, bollard::errors::Error> {
    let container_config = bollard::container::Config {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        args_escaped: None,
        image: Some(image),
        working_dir: Some(working_dir),
        cmd: Some(cmd),
        env: None,
        stop_timeout: Some(TIMEOUT as isize),
        host_config,
        ..Default::default()
    };
    docker
        .create_container(None::<CreateContainerOptions<&str>>, container_config)
        .await
}

pub async fn start_and_log_container(
    container_id: &str,
    docker: &bollard::Docker,
) -> Result<ScriptOutput, Error> {
    docker
        .start_container(container_id, None::<StartContainerOptions<String>>)
        .await?;
    let mut wait_stream = docker.wait_container(
        &container_id,
        Some(WaitContainerOptions {
            condition: "not-running",
        }),
    );

    let status_code = wait_stream.next().await.unwrap().unwrap().status_code;
    let (stdout, stderr) = get_output(container_id, &docker).await;

    Ok(ScriptOutput {
        stdout,
        stderr,
        status_code,
    })
}

async fn get_output(container_id: &str, docker: &bollard::Docker) -> (String, String) {
    let log_opt = Some(LogsOptions {
        stdout: true,
        stderr: true,
        ..Default::default()
    });
    let mut output_stream = docker.logs(container_id, log_opt);
    let mut stdout = String::new();
    let mut stderr = String::new();
    while let Some(out) = output_stream.next().await {
        match out.unwrap() {
            LogOutput::StdOut { message } => write!(stdout, "{}\n", message).unwrap(),
            LogOutput::StdErr { message } => write!(stderr, "{}\n", message).unwrap(),
            _ => (),
        }
    }
    (stdout, stderr)
}

impl From<bollard::errors::Error> for Error {
    fn from(err: bollard::errors::Error) -> Self {
        Error::Docker(err.to_string())
    }
}
