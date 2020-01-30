use crate::crash_test::Error;
use crate::docker_api::{
    create_container, create_host_config, docker_image, docker_mount_points,
    start_and_log_container, Mount,
};
use bollard::container::RemoveContainerOptions;
use grpc_api::Script;
use std::path::Path;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub async fn run(
    script: &Script,
    script_path: &Path,
    dir: &Path,
    args_from_conf: &Vec<String>,
) -> Result<ScriptOutput, Error> {
    let (prog, mut args) = script.command_line();
    let dur = Duration::from_secs(30);

    #[cfg(target_family = "windows")]
    {
        args.push(fix_windows_path(&script, &script_path));
    }

    #[cfg(target_family = "unix")]
    {
        args.push(script_path.to_path_buf());
    }
    dbg!(&args);
    let out = timeout(
        dur,
        Command::new(prog)
            .current_dir(dir)
            .args(args)
            .args(args_from_conf)
            .output(),
    )
    .await
    .map_err(|e| Error::Timeout(e, dur.into()))?;
    dbg!(&out);
    let out = match out {
        Err(_) => panic!("Command {} not found!", prog),
        Ok(out) => out,
    };
    exited_fine(&out)?;
    Ok(ScriptOutput {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status_code: out.status.code().unwrap() as u64,
    })
}

pub async fn run_router(
    docker: &bollard::Docker,
    script: &Script,
    script_path: &Path,
    out_dir: &Path,
    args_from_conf: &Vec<String>,
) -> Result<ScriptOutput, Error> {
    //because windows
    #[cfg(target_family = "windows")]
    let script_dir = {
        script_path
            .parent()
            .unwrap()
            .to_string_lossy()
            .replace("\\\\?\\", "")
    };

    #[cfg(target_family = "unix")]
    let script_dir = { script_path.parent().unwrap() };

    run_in_container(
        docker,
        &script,
        script_path
            .to_path_buf()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        script_dir.as_ref(),
        &out_dir,
        &args_from_conf,
    )
    .await
}

async fn run_in_container(
    docker: &bollard::Docker,
    script: &Script,
    script_name: &str,
    script_dir: &Path,
    out_dir: &Path,
    args_from_conf: &Vec<String>,
) -> Result<ScriptOutput, Error> {
    let (inner_working_dir, inner_script_dir) = docker_mount_points(script);
    let out_dir_mount = Mount {
        source_dir: out_dir.to_str().unwrap(),
        target_dir: inner_working_dir,
    };

    let script_dir_mount = Mount {
        source_dir: script_dir.to_str().unwrap(),
        target_dir: inner_script_dir,
    };
    let host_config = create_host_config(&out_dir_mount, &script_dir_mount);

    // TODO fix me
    let (prog, _args) = script.command_line(); //TODO rm _args
    let inner_script_path = [inner_script_dir, script_name].join("");
    let mut cmd = vec![prog, inner_script_path.as_ref()];
    let mut args2: Vec<&str> = args_from_conf.iter().map(AsRef::as_ref).collect();
    cmd.append(args2.as_mut());
    // TODO fix me

    let container = create_container(
        cmd,
        docker_image(&script),
        host_config,
        inner_working_dir,
        &docker,
    )
    .await
    .expect("cant crate container");
    let dur = Duration::from_secs(60);
    let out = timeout(dur, start_and_log_container(&container.id, &docker))
        .await
        .map_err(|e| {
            let err = Error::Timeout(e, dur.into());
            log::info!("{}", &err);
            err
        })?;

    docker
        .remove_container(
            &container.id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .expect("error delete container");
    dbg!(&out);
    Ok(out)
}

/*impl TryFrom<Output> for ScriptOutput {
    type Error = Error;

    fn try_from(o: Output) -> Result<Self, Error> {
/*        Ok(ScriptOutput {
            stdout: String::from_utf8(o.stdout.clone())?,
            o*/
        })
    }
}*/
#[derive(Debug, Clone)]
pub struct ScriptOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: u64,
}

fn exited_fine(out: &Output) -> Result<(), Error> {
    if out.status.success() && out.stderr.is_empty() {
        Ok(())
    } else {
        Err(Error::ExitCode(
            String::from_utf8(out.stderr.clone()).unwrap_or_default(),
        ))
    }
}

#[cfg(target_family = "windows")]
fn fix_windows_path(script: &Script, script_path: &Path) -> std::ffi::OsString {
    use path_slash::PathExt;
    use regex::{Captures, Regex};
    if script == &Script::Bash || script == &Script::Shell {
        let str = script_path.to_slash_lossy().replace("\\\\?\\", "");
        let re = Regex::new(r"^([A-Z])://").unwrap();
        re.replace(&str, |caps: &Captures| {
            format!("/mnt/{}/", caps[1].to_ascii_lowercase())
        })
        .to_string()
        .into()
    } else {
        script_path.into()
    }
}
