//! The errors of the contract: a kind ion-repo can act on, a message for people.

use std::fmt;

use indexmap::IndexMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Request,
    Template,
    Ref,
    Network,
    Answers,
    Project,
    Io,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Request => "request",
            Kind::Template => "template",
            Kind::Ref => "ref",
            Kind::Network => "network",
            Kind::Answers => "answers",
            Kind::Project => "project",
            Kind::Io => "io",
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub kind: Kind,
    pub message: String,
    /// For `answers`: each refused answer and its validator's message.
    pub questions: IndexMap<String, String>,
}

impl Error {
    pub fn new(kind: Kind, message: impl Into<String>) -> Error {
        Error {
            kind,
            message: message.into(),
            questions: IndexMap::new(),
        }
    }

    pub fn answer(question: &str, message: impl Into<String>) -> Error {
        let message = message.into();
        let mut questions = IndexMap::new();
        questions.insert(question.to_string(), message.clone());
        Error {
            kind: Kind::Answers,
            message: format!("invalid answer to {question}: {message}"),
            questions,
        }
    }

    pub fn template(message: impl Into<String>) -> Error {
        Error::new(Kind::Template, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.name(), self.message)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
