use std::fmt;
use std::path::{Path, PathBuf};
use std::time;

use async_trait::async_trait;
use futures::pin_mut;
use futures::StreamExt;
use log::info;
use tokio::fs;

use crate::docker_api::ScriptOutput;
use crate::fs_util;

#[async_trait]
pub trait CrashTester: Sync + Send {
    async fn test(&self) -> Result<(), Error>;
}
pub struct Stdout {
    expected: ScriptOutput,
    testet: ScriptOutput,
}

impl Stdout {
    pub fn boxed(expected: ScriptOutput, testet: ScriptOutput) -> Box<dyn CrashTester> {
        Box::new(Stdout { expected, testet })
    }
}

pub struct Files {
    expected_dir: PathBuf,
    given_dir: PathBuf,
}
impl Files {
    pub fn boxed(a: PathBuf, b: PathBuf) -> Box<dyn CrashTester> {
        Box::new(Files {
            expected_dir: a,
            given_dir: b,
        })
    }
    fn cmp_file_type(&self, a: &Path, b: &Path) -> bool {
        (a.is_file() && b.is_file()) || (a.is_dir() && b.is_dir())
    }
}

#[async_trait]
impl CrashTester for Stdout {
    async fn test(&self) -> Result<(), Error> {
        let stdout = trim_new_lines(&self.testet.stdout);

        if !self.testet.stderr.is_empty() || self.testet.status_code > 0 {
            //maybe bad syntax
            return Err(Error::ExitCode(self.testet.stderr.clone()));
        }
        let expected_output = trim_new_lines(&self.expected.stdout); // check if solution is also no error
        log::info!("expected stdout: {:#?}", expected_output);
        log::info!("result stdout: {:#?}", stdout);
        if expected_output.contains(&stdout) {
            Ok(())
        } else {
            Err(Error::WrongOutput(format!(
                "expected stdout:({:#?}) result stdout:({:#?})",
                expected_output, stdout
            )))
        }
    }
}

#[async_trait]
impl CrashTester for Files {
    async fn test(&self) -> Result<(), Error> {
        print_dir_content("expected dir:", &self.expected_dir).await?;
        print_dir_content("dir after test:", &self.given_dir).await?;
        let stream = fs_util::ls_dir_content(self.expected_dir.clone());
        pin_mut!(stream);
        while let Some(Ok(solution_entry)) = stream.next().await {
            let path_to_check = &self.given_dir.as_path().join(
                solution_entry.strip_prefix(&self.expected_dir).unwrap(), // TODO err handling
            );
            if path_to_check.exists()
                && self.cmp_file_type(&solution_entry, &path_to_check.as_path())
            {
                if solution_entry.is_file() {
                    let solution_content =
                        trim_new_lines(&fs::read_to_string(&solution_entry).await?);
                    let result_content = trim_new_lines(&fs::read_to_string(&path_to_check).await?);
                    if solution_content != result_content {
                        return Err(Error::ExpectedFileNotSame(solution_content, result_content));
                    }
                }
            } else {
                return Err(Error::ExpectedDirNotSame);
            }
        }

        Ok(())
    }
}

async fn print_dir_content(msg: &str, root: &Path) -> Result<(), Error> {
    info!("{}", &msg);
    let stream = fs_util::ls_dir_content(root.to_path_buf().clone());
    pin_mut!(stream);
    while let Some(Ok(entry)) = stream.next().await {
        info!("    path: {}", &entry.display());
        if entry.is_file() {
            let content = fs::read_to_string(&entry).await.unwrap_or_default();
            info!("    file content: {:#?}", &content);
        }
    }
    Ok(())
}

#[derive(Debug, failure::Fail, derive_more::From)]
pub enum Error {
    #[fail(display = "Time out reached! Script took more than {}.", _1)]
    Timeout(tokio::time::Elapsed, DurationDisplay),
    #[from]
    #[fail(display = "Script produced invalid UFT8.")]
    NoUTF8(std::string::FromUtf8Error),
    #[fail(display = "Does not contains expected output. {}", _0)]
    WrongOutput(String),
    #[fail(display = "Solution dir and tested dir have not the same content")]
    ExpectedDirNotSame,
    #[fail(display = "Script finished with exit code 1 stderr: {}", _0)]
    ExitCode(String),
    #[fail(display = "Wrong file content: expected({:#?}) result({:#?})", _0, _1)]
    ExpectedFileNotSame(String, String),
    #[fail(display = "Can't create temp file. {}", _0)]
    CantCreatTempFile(std::io::Error),
    #[from]
    #[fail(display = "Could not copy included files for testing {}", _0)]
    Copy(std::io::Error),
    #[fail(display = "IO error while reading the dir {:?}", _0)]
    ListDir(PathBuf),
    #[fail(display = "Docker error {}", _0)]
    Docker(String),
}
#[derive(Debug, derive_more::From)]
pub struct DurationDisplay(time::Duration);

impl fmt::Display for DurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} seconds", self.0.as_secs())
    }
}

pub fn trim_new_lines(s: &str) -> String {
    s.chars()
        .filter(|&c| c != '\r')
        .collect::<String>()
        .lines()
        .map(|line| {
            let mut n_line = line.trim_end().to_string();
            n_line.push('\n');
            n_line
        })
        .collect::<String>()
}
