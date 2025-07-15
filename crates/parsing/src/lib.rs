use std::io::Read;

use thiserror::Error;
mod json;
mod types;
mod yaml;

use tree_sitter::LanguageError;
pub use types::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not detect file type")]
    NoFileType,

    #[error("parsing error: {0}")]
    Parsing(#[from] ParsingError),
}
#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("setting tree-sitter language")]
    SettingLanguage(#[from] LanguageError),
}

pub type Result<T> = std::result::Result<T, Error>;

enum FileType {
    Json,
    Yaml,
}

fn detect_file_type(content: &str) -> Option<FileType> {
    // TODO
    Some(FileType::Yaml)
}

pub fn parse(content: &str) -> Result<Targets> {
    match detect_file_type(content) {
        Some(FileType::Json) => json::parse(content).map_err(Error::Parsing),
        Some(FileType::Yaml) => yaml::parse(content).map_err(Error::Parsing),
        None => Err(Error::NoFileType),
    }
}

pub fn parse_from(mut reader: impl Read) -> Result<Targets> {
    let mut content = String::new();
    reader.read_to_string(&mut content).map_err(Error::Io)?;
    parse(&content)
}
