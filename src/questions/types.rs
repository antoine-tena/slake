//! A question's type, and Copier's `cast_answer` to it.

use super::Question;
use crate::cast::{cast_to_bool, py_float, py_int};
use crate::data::Data;
use crate::error::{Error, Result};

const TYPES: [&str; 7] = ["bool", "float", "int", "json", "str", "yaml", "path"];

impl Question<'_> {
    pub(super) fn type_name(&self) -> Result<String> {
        let declared = match self.field("type") {
            Some(value) => self.render(value)?.to_string(),
            None => String::new(),
        };
        // Copier infers an undeclared type from the default's Python type.
        let name = if declared.is_empty() {
            match self.field("default") {
                Some(Data::Bool(_)) => "bool",
                Some(Data::Int(_)) => "int",
                Some(Data::Float(_)) => "float",
                Some(Data::Str(_)) => "str",
                _ => "yaml",
            }
            .to_string()
        } else {
            declared
        };
        if TYPES.contains(&name.as_str()) {
            Ok(name)
        } else {
            let message = format!("unsupported type {name:?} in question {}", self.name);
            Err(Error::template(message))
        }
    }

    /// Copier's `cast_answer`.
    pub(super) fn cast(&self, answer: &Data) -> Result<Data> {
        let type_name = self.type_name()?;
        let invalid = || {
            let message = format!("invalid answer {answer} for type {type_name}");
            Error::answer(self.name, message)
        };
        if matches!(answer, Data::Null) && !["json", "yaml"].contains(&type_name.as_str()) {
            return Err(invalid());
        }
        let cast = match type_name.as_str() {
            "bool" => Some(Data::Bool(cast_to_bool(answer))),
            "int" => to_int(answer).map(Data::Int),
            "float" => to_float(answer).map(Data::Float),
            "str" => to_str(answer),
            "path" => Some(Data::Str(answer.to_string())),
            "json" => return self.read_json(answer),
            _ => return self.read_yaml(answer),
        };
        cast.ok_or_else(invalid)
    }

    fn read_json(&self, answer: &Data) -> Result<Data> {
        match answer {
            Data::Str(text) => serde_json::from_str::<serde_json::Value>(text)
                .map(|value| Data::from_json(&value))
                .map_err(|error| Error::answer(self.name, error.to_string())),
            other => Ok(other.clone()),
        }
    }

    fn read_yaml(&self, answer: &Data) -> Result<Data> {
        match answer {
            Data::Str(text) => {
                crate::yaml::load(text).map_err(|error| Error::answer(self.name, error))
            }
            other => Ok(other.clone()),
        }
    }
}

/// Python's `int(answer)`.
fn to_int(answer: &Data) -> Option<i64> {
    match answer {
        Data::Int(value) => Some(*value),
        Data::Bool(value) => Some(i64::from(*value)),
        Data::Float(value) if value.is_finite() => Some(value.trunc() as i64),
        Data::Str(text) => py_int(text),
        _ => None,
    }
}

/// Python's `float(answer)`.
fn to_float(answer: &Data) -> Option<f64> {
    match answer {
        Data::Int(value) => Some(*value as f64),
        Data::Bool(value) => Some(f64::from(u8::from(*value))),
        Data::Float(value) => Some(*value),
        Data::Str(text) => py_float(text),
        _ => None,
    }
}

/// Python's `str(answer)`, for the scalars Copier accepts as a string.
fn to_str(answer: &Data) -> Option<Data> {
    match answer {
        Data::Str(_) => Some(answer.clone()),
        Data::Int(_) | Data::Float(_) | Data::Bool(_) => Some(Data::Str(answer.to_string())),
        _ => None,
    }
}
