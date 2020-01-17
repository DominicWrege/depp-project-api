use crate::crash_test::Error;
use grpc_api::Script;
use std::convert::TryFrom;
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
    let (prog, mut args) = script.commandline();
    let dur = Duration::from_secs(30);
    args.push(script_path.to_path_buf());
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

    let out = match out {
        Err(_) => panic!("Command {} not found!", prog),
        Ok(out) => out,
    };
    exited_fine(&out)?;
    ScriptOutput::try_from(out)
}

impl TryFrom<Output> for ScriptOutput {
    type Error = Error;

    fn try_from(o: Output) -> Result<Self, Error> {
        Ok(ScriptOutput {
            stdout: String::from_utf8(o.stdout.clone())?,
            output: o,
        })
    }
}
pub struct ScriptOutput {
    pub stdout: String,
    pub output: Output,
}

// #[cfg(any(unix))]
// impl Script {
//     pub fn commandline(&self) -> (&'static str, Vec<PathBuf>) {
//         match self {
//             Script::PowerShell => ("pwsh", vec![]),
//             Script::Shell => ("sh", vec![]),
//             Script::Batch => ("wine", vec!["cmd.exe".into(), "/C".into()]),
//             Script::Python3 => ("python3", vec![]),
//             Script::Bash | Script::Awk | Script::Sed | _ => ("bash", vec![]),
//         }
//     }
// }

// impl From<i32> for Script {
//     fn from(n: i32) -> Self {
//         match n {
//             1 => Script::PowerShell,
//             3 => Script::Python3,
//             4 => Script::Shell,
//             5 => Script::Bash,
//             6 => Script::Awk,
//             7 => Script::Sed,
//             2 | _ => Script::Batch,
//         }
//     }
// }

// #[cfg(target_os = "windows")]
// impl Script {
//     pub fn commandline(&self) -> (&'static str, Vec<PathBuf>) {
//         match self {
//             Script::PowerShell => ("powershell.exe", vec![]),
//             Script::Shell => ("sh", vec![]),
//             Script::Batch => ("cmd.exe", vec!["/C".into()]),
//             Script::Python3 => ("python3", vec![]),
//             Script::Bash | Script::Awk | Script::Sed | _ => ("bash", vec![]),
//         }
//     }
// }

// impl Script {
//     pub fn file_extension(&self) -> &'static str {
//         match self {
//             Script::Batch => ".bat",
//             Script::PowerShell => ".ps1",
//             Script::Python3 => ".py",
//             Script::Shell | Script::Bash | Script::Sed | Script::Awk | _ => ".sh",
//         }
//     }
//     pub async fn run(
//         &self,
//         script_path: &Path,
//         dir: &Path,
//         args_from_conf: &Vec<String>,
//     ) -> Result<ScriptOutput, Error> {
//         let (prog, mut args) = self.commandline();
//         let dur = Duration::from_secs(30);
//         args.push(script_path.to_path_buf());
//         let out = timeout(
//             dur,
//             Command::new(prog)
//                 .current_dir(dir)
//                 .args(args)
//                 .args(args_from_conf)
//                 .output(),
//         )
//         .await
//         .map_err(|e| Error::Timeout(e, dur.into()))?;

//         let out = match out {
//             Err(_) => panic!("Command {} not found!", prog),
//             Ok(out) => out,
//         };
//         exited_fine(&out)?;
//         ScriptOutput::try_from(out)
//     }
// }

// impl TryFrom<Output> for ScriptOutput {
//     type Error = Error;

//     fn try_from(o: Output) -> Result<Self, Error> {
//         Ok(ScriptOutput {
//             stdout: String::from_utf8(o.stdout.clone())?,
//             output: o,
//         })
//     }
// }

fn exited_fine(out: &Output) -> Result<(), Error> {
    if out.status.success() && out.stderr.is_empty() {
        Ok(())
    } else {
        Err(Error::ExitCode(
            String::from_utf8(out.stderr.clone()).unwrap_or_default(),
        ))
    }
}