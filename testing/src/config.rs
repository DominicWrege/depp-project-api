use crate::deep_project::Script;
use serde::{de, Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub name: String,
    pub assignment: Vec<Assignment>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Assignment {
    pub name: String,
    #[serde(deserialize_with = "into_absolute_path")]
    pub solution_path: PathBuf,
    #[serde(default)]
    pub include_files: Vec<PathBuf>,
    #[serde(default)]
    #[serde(rename = "type")]
    pub script_type: Script,
    #[serde(default)]
    pub args: Vec<String>,
    //pub script_contains: Option<Pattern>, //
}

fn into_absolute_path<'de, D>(deserial: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let relative_path = PathBuf::deserialize(deserial)?;

    fs::canonicalize(relative_path).map_err(de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Pattern {
    #[serde(default)]
    pub regex: bool,
    pub text: String,
}

#[derive(Debug, err_derive::Error, derive_more::From)]
pub enum Error {
    #[from]
    #[error(display = "can`t find config file, {}", _0)]
    ConfigFile(std::io::Error),
    #[from]
    #[error(display = "wrong toml format, {}", _0)]
    Toml(toml::de::Error),
}

pub fn parse_config(path: &Path) -> Result<HashMap<AssignmentId, Assignment>, Error> {
    let file_content = fs::read_to_string(path)?;
    let exercise = toml::from_str(&file_content)?;
    Ok(into_config_map(exercise))
}

fn into_config_map(conf: Config) -> HashMap<AssignmentId, Assignment> {
    conf.assignment
        .into_iter()
        .map(|assignment| {
            for path in &assignment.include_files {
                path_exists_and_is_file(&path);
            }
            path_exists_and_is_file(&assignment.solution_path);
            (Uuid::new_v4().into(), assignment)
        })
        .collect::<HashMap<AssignmentId, Assignment>>()
}

fn path_exists_and_is_file(p: &Path) {
    if !p.exists() || p.is_dir() {
        panic!(
            "Config error: path to {:#?} does not exists or is not a file.",
            p
        )
    }
}

// TODO remove this later and use it
#[derive(
    Debug, Clone, Hash, Eq, PartialEq, Deserialize, serde::Serialize, Copy, derive_more::From,
)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentId(pub Uuid);

// TODO remove this later and use it
