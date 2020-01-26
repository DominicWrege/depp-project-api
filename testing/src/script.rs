use crate::crash_test::Error;
use crate::docker::{create_container, create_host_config, start_container, Mount};
use bollard::container::RemoveContainerOptions;
use grpc_api::Script;
use std::convert::TryFrom;
use std::fmt::Write;
use std::path::Path;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub async fn run_router(
    script: &Script,
    script_path: &Path,
    out_dir: &Path,
    args_from_conf: &Vec<String>,
) -> Result<ScriptOutput, Error> {
    match script {
        Script::PowerShell | Script::Batch => {
            run(&script, &script_path, &out_dir, &args_from_conf).await
        }
        _ => {
            run_in_container(
                &script,
                script_path
                    .to_path_buf()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                &script_path.parent().unwrap(),
                &out_dir,
            )
            .await
        }
    }
}

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
    })
}

async fn run_in_container(
    script: &Script,
    script_name: &str,
    script_dir: &Path,
    out_dir: &Path,
) -> Result<ScriptOutput, Error> {
    let out_dir_mount = Mount {
        source_dir: out_dir.to_str().unwrap(),
        target_dir: "/testing".as_ref(),
    };

    let script_dir_mount = Mount {
        source_dir: script_dir.to_str().unwrap(),
        target_dir: "/script_dir".as_ref(),
    };
    let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    let host_config = create_host_config(&out_dir_mount, &script_dir_mount);
    let container = create_container(
        "/testing".as_ref(),
        script_name,
        host_config,
        &docker,
        &script,
    )
    .await
    .unwrap();
    let out = start_container(&container.id, &docker).await;
    dbg!(&out);
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
