//! The settings and questions of a `copier.yml`, refused when they ask for a part of Copier
//! slake does not have.

use crate::data::{Data, Map};
use crate::error::{Error, Result};
use crate::template::{ANSWERS_FILE, SUBDIRECTORY, SUFFIX};
use crate::yaml;

pub struct Config {
    pub questions: Vec<(String, Map)>,
    pub skip_if_exists: Vec<String>,
}

/// Settings kiln's template does not use: refused unless empty.
const UNSUPPORTED: [&str; 9] = [
    "envops",
    "exclude",
    "external_data",
    "jinja_extensions",
    "migrations",
    "preserve_symlinks",
    "secret_questions",
    "tasks",
    "use_prereleases",
];

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        let mut settings = Map::new();
        let mut questions = Vec::new();
        for (key, value) in merged_documents(text)? {
            let Data::Str(key) = key else {
                return Err(Error::template(format!(
                    "copier.yml: the key {key} is not a string"
                )));
            };
            match key.strip_prefix('_') {
                Some(setting) => settings.insert(Data::str(setting), value),
                None => {
                    let details = question_details(&key, value)?;
                    questions.push((key, details));
                }
            }
        }
        check_settings(&settings)?;
        Ok(Config {
            questions,
            skip_if_exists: skip_if_exists(&settings)?,
        })
    }

    pub fn has_question(&self, name: &str) -> bool {
        self.questions.iter().any(|(question, _)| question == name)
    }
}

/// Copier's `ChainMap` of the documents: a later document's key wins, the first place a key
/// appears stays.
fn merged_documents(text: &str) -> Result<Map> {
    let documents =
        yaml::load_all(text).map_err(|error| Error::template(format!("copier.yml: {error}")))?;
    let mut merged = Map::new();
    for document in documents {
        match document {
            Data::Null => {}
            Data::Map(map) => {
                for (key, value) in map {
                    merged.insert(key, value);
                }
            }
            _ => {
                return Err(Error::template(
                    "copier.yml: each document must be a mapping",
                ));
            }
        }
    }
    Ok(merged)
}

/// A question's mapping; a bare value is its default.
fn question_details(key: &str, value: Data) -> Result<Map> {
    let details = match value {
        Data::Map(details) => details,
        other => Map::from_iter([(Data::str("default"), other)]),
    };
    for flag in ["secret", "multiselect"] {
        if details
            .get(flag)
            .is_some_and(|value| !matches!(value, Data::Bool(false) | Data::Null))
        {
            return Err(Error::template(format!(
                "question {key}: `{flag}` is not supported"
            )));
        }
    }
    Ok(details)
}

fn check_settings(settings: &Map) -> Result<()> {
    setting_is(settings, "subdirectory", SUBDIRECTORY)?;
    setting_is(settings, "templates_suffix", SUFFIX)?;
    setting_is(settings, "answers_file", ANSWERS_FILE)?;
    for key in UNSUPPORTED {
        match settings.get(key) {
            None | Some(Data::Null) | Some(Data::Bool(false)) => {}
            Some(Data::List(items)) if items.is_empty() => {}
            Some(Data::Map(map)) if map.is_empty() => {}
            Some(_) => return Err(Error::template(format!("_{key} is not supported"))),
        }
    }
    Ok(())
}

fn setting_is(settings: &Map, key: &str, expected: &str) -> Result<()> {
    match settings.get(key) {
        None => Ok(()),
        Some(Data::Str(value)) if value == expected => Ok(()),
        Some(value) => Err(Error::template(format!(
            "_{key}: {value} is not supported, only {expected}"
        ))),
    }
}

fn skip_if_exists(settings: &Map) -> Result<Vec<String>> {
    match settings.get("skip_if_exists") {
        None | Some(Data::Null) => Ok(Vec::new()),
        Some(Data::List(items)) => Ok(items.iter().map(ToString::to_string).collect()),
        Some(Data::Str(item)) => Ok(vec![item.clone()]),
        Some(_) => Err(Error::template("_skip_if_exists must be a list")),
    }
}
