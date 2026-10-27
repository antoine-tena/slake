//! The JSON contract with kiln (`docs/contract.md`): a request read from standard input and
//! checked before use, a response written to standard output.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map as JsonMap, Value as Json};

use crate::data::{Data, Map};
use crate::error::{Error, Kind, Result};

/// The version of the contract this slake speaks.
pub const CONTRACT: u64 = 1;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where a template comes from, as the request gives it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateFields {
    url: Option<String>,
    #[serde(rename = "ref")]
    reference: Option<String>,
    path: Option<PathBuf>,
    commit: Option<String>,
}

/// A template: a revision of a git repository, or a directory as it is on disk.
pub enum Template {
    Git {
        url: String,
        reference: String,
    },
    Directory {
        path: PathBuf,
        commit: Option<String>,
    },
}

impl TemplateFields {
    pub fn template(self) -> Result<Template> {
        let request = |message: &str| Error::new(Kind::Request, format!("template: {message}"));
        match (self.url, self.reference, self.path, self.commit) {
            (Some(url), Some(reference), None, None) => {
                if url.is_empty() || reference.is_empty() {
                    return Err(request("`url` and `ref` cannot be empty"));
                }
                Ok(Template::Git { url, reference })
            }
            (None, None, Some(path), commit) => Ok(Template::Directory { path, commit }),
            _ => Err(request(
                "either `url` and `ref`, or `path` and an optional `commit`",
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderRequest {
    pub contract: u64,
    pub template: TemplateFields,
    #[serde(default)]
    pub answers: JsonMap<String, Json>,
    #[serde(default)]
    pub defaults: JsonMap<String, Json>,
    pub destination: PathBuf,
    pub cache: Option<PathBuf>,
    #[serde(default)]
    pub offline: bool,
}

/// A request of `command`, checked against its schema and the contract's version.
pub fn parse<T: DeserializeOwned + Versioned>(input: &str) -> Result<T> {
    let request: T = serde_json::from_str(input)
        .map_err(|error| Error::new(Kind::Request, format!("invalid request: {error}")))?;
    if request.contract() != CONTRACT {
        return Err(Error::new(
            Kind::Request,
            format!(
                "contract {} is not supported: this slake speaks contract {CONTRACT}",
                request.contract()
            ),
        ));
    }
    Ok(request)
}

pub trait Versioned {
    fn contract(&self) -> u64;
}

impl Versioned for RenderRequest {
    fn contract(&self) -> u64 {
        self.contract
    }
}

/// Answers as the request gives them, keys in their order.
pub fn answers(fields: &JsonMap<String, Json>) -> Map {
    fields
        .iter()
        .map(|(key, value)| (Data::str(key.as_str()), Data::from_json(value)))
        .collect()
}

/// A path of the request, made absolute against the working directory.
pub fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path)
        .map_err(|error| Error::new(Kind::Request, format!("{}: {error}", path.display())))
}

/// A response: `contract` and `slake` first, then the command's fields.
pub fn success(fields: JsonMap<String, Json>) -> Json {
    let mut response = header();
    response.extend(fields);
    Json::Object(response)
}

pub fn failure(error: &Error) -> Json {
    let mut details = JsonMap::new();
    details.insert("kind".into(), error.kind.name().into());
    details.insert("message".into(), error.message.clone().into());
    if !error.questions.is_empty() {
        let questions = error
            .questions
            .iter()
            .map(|(name, message)| (name.clone(), Json::from(message.as_str())));
        details.insert("questions".into(), Json::Object(questions.collect()));
    }
    let mut response = header();
    response.insert("error".into(), Json::Object(details));
    Json::Object(response)
}

fn header() -> JsonMap<String, Json> {
    let mut header = JsonMap::new();
    header.insert("contract".into(), CONTRACT.into());
    header.insert("slake".into(), VERSION.into());
    header
}
