//! Answers and settings as Python sees them: what PyYAML and `json` load, and what Copier casts
//! and records.

use std::fmt;

use minijinja::Value;

#[derive(Clone, Debug)]
pub enum Data {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// A YAML date (`2026-10-26`, unquoted): PyYAML reads it as a `datetime.date`.
    Date(String),
    List(Vec<Data>),
    Map(Map),
}

/// A Python `dict`: insertion order, a key assigned again keeps its place.
#[derive(Clone, Debug, Default)]
pub struct Map(Vec<(Data, Data)>);

impl Map {
    pub fn new() -> Map {
        Map(Vec::new())
    }

    pub fn insert(&mut self, key: Data, value: Data) {
        match self.0.iter_mut().find(|(existing, _)| existing.py_eq(&key)) {
            Some(entry) => entry.1 = value,
            None => self.0.push((key, value)),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Data> {
        self.0
            .iter()
            .find(|(existing, _)| matches!(existing, Data::Str(name) if name == key))
            .map(|(_, value)| value)
    }

    pub fn remove(&mut self, key: &str) -> Option<Data> {
        let index = self
            .0
            .iter()
            .position(|(existing, _)| matches!(existing, Data::Str(name) if name == key))?;
        Some(self.0.remove(index).1)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Data, Data)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for Map {
    type Item = (Data, Data);
    type IntoIter = std::vec::IntoIter<(Data, Data)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(Data, Data)> for Map {
    fn from_iter<I: IntoIterator<Item = (Data, Data)>>(iter: I) -> Map {
        let mut map = Map::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl Data {
    pub fn str(text: impl Into<String>) -> Data {
        Data::Str(text.into())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Data::Str(text) => Some(text),
            _ => None,
        }
    }

    /// Python's `==`: `1 == 1.0 == True`.
    pub fn py_eq(&self, other: &Data) -> bool {
        match (self.as_number(), other.as_number()) {
            (Some(left), Some(right)) => return left == right,
            (Some(_), None) | (None, Some(_)) => return false,
            (None, None) => {}
        }
        match (self, other) {
            (Data::Null, Data::Null) => true,
            (Data::Str(left), Data::Str(right)) | (Data::Date(left), Data::Date(right)) => {
                left == right
            }
            (Data::List(left), Data::List(right)) => {
                left.len() == right.len() && left.iter().zip(right).all(|(a, b)| a.py_eq(b))
            }
            (Data::Map(left), Data::Map(right)) => {
                left.len() == right.len()
                    && left.iter().all(|(key, value)| {
                        right.iter().any(|(other_key, other_value)| {
                            key.py_eq(other_key) && value.py_eq(other_value)
                        })
                    })
            }
            _ => false,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match *self {
            Data::Bool(value) => Some(f64::from(u8::from(value))),
            Data::Int(value) => Some(value as f64),
            Data::Float(value) => Some(value),
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Data::Null => Value::from(()),
            Data::Bool(value) => Value::from(*value),
            Data::Int(value) => Value::from(*value),
            Data::Float(value) => Value::from(*value),
            Data::Str(text) | Data::Date(text) => Value::from(text.as_str()),
            Data::List(items) => Value::from(items.iter().map(Data::to_value).collect::<Vec<_>>()),
            Data::Map(map) => Value::from_pairs(
                map.iter()
                    .map(|(key, value)| (key.to_value(), value.to_value())),
            ),
        }
    }

    pub fn from_json(value: &serde_json::Value) -> Data {
        match value {
            serde_json::Value::Null => Data::Null,
            serde_json::Value::Bool(value) => Data::Bool(*value),
            serde_json::Value::Number(number) => match number.as_i64() {
                Some(value) => Data::Int(value),
                None => Data::Float(number.as_f64().unwrap_or(f64::NAN)),
            },
            serde_json::Value::String(text) => Data::Str(text.clone()),
            serde_json::Value::Array(items) => {
                Data::List(items.iter().map(Data::from_json).collect())
            }
            serde_json::Value::Object(object) => Data::Map(
                object
                    .iter()
                    .map(|(key, value)| (Data::Str(key.clone()), Data::from_json(value)))
                    .collect(),
            ),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Data::Null => serde_json::Value::Null,
            Data::Bool(value) => serde_json::Value::Bool(*value),
            Data::Int(value) => serde_json::Value::from(*value),
            Data::Float(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Data::Str(text) | Data::Date(text) => serde_json::Value::String(text.clone()),
            Data::List(items) => {
                serde_json::Value::Array(items.iter().map(Data::to_json).collect())
            }
            Data::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(key, value)| (key.to_string(), value.to_json()))
                    .collect(),
            ),
        }
    }
}

/// Python's `str()`.
impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Data::Str(text) | Data::Date(text) => f.write_str(text),
            other => f.write_str(&crate::pyfmt::repr_data(other)),
        }
    }
}
