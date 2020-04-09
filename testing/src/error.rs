use std::path::PathBuf;
use std::{fmt, time};

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
    #[fail(display = "Regex error {}", _0)]
    InvalidRegex(String),
    #[fail(display = "No Regex match found in '{}' for regex: '{}'", _0, _1)]
    NoRegexMatch(String, regex::Regex),
    #[fail(display = "Stdout is not sorted. stdout: {:#?}", _0)]
    NoSorted(String),
    #[fail(
        display = "Custom script wrote to stderr or exit is not 0. Stderr: {}",
        _0
    )]
    CustomScript(String),
    #[fail(display = "Failed to run the custom scrip. Error: {}", _0)]
    FailedRunCustomScript(std::io::Error),
}
#[derive(Debug, derive_more::From)]
pub struct DurationDisplay(time::Duration);

impl fmt::Display for DurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} seconds", self.0.as_secs())
    }
}