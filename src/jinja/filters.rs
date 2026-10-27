//! Jinja2's filters with Python's semantics where minijinja's differ, and Copier's `pathjoin`.

use std::borrow::Cow;

use minijinja::value::{Kwargs, Rest, Value, ValueKind, ValueOrKwargs};
use minijinja::{Environment, Error, ErrorKind, State};

use super::methods::is_space;
use crate::pyfmt::{json_dumps, str_value};

pub fn register(env: &mut Environment<'static>) {
    env.add_filter("capitalize", capitalize);
    env.add_filter("e", escape);
    env.add_filter("escape", escape);
    env.add_filter("join", join);
    env.add_filter("length", length);
    env.add_filter("count", length);
    env.add_filter("list", list);
    env.add_filter("lower", lower);
    env.add_filter("upper", upper);
    env.add_filter("replace", replace);
    env.add_filter("select", select);
    env.add_filter("string", string);
    env.add_filter("title", title);
    env.add_filter("tojson", tojson);
    env.add_filter("trim", trim);
    env.add_filter("unique", unique);
    super::ansible::register(env);
}

pub(super) fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidOperation, message.into())
}

/// Iterating as Python does: a string gives its characters, a dict its keys, `Undefined`
/// nothing.
fn py_iter(value: &Value) -> Result<Vec<Value>, Error> {
    match value.kind() {
        ValueKind::Undefined => Ok(Vec::new()),
        ValueKind::String => Ok(value
            .as_str()
            .unwrap_or_default()
            .chars()
            .map(|ch| Value::from(ch.to_string()))
            .collect()),
        ValueKind::Seq | ValueKind::Iterable | ValueKind::Map => Ok(value.try_iter()?.collect()),
        kind => Err(invalid(format!("'{kind}' object is not iterable"))),
    }
}

/// Python's `str.capitalize()`.
pub fn capitalize_str(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.as_str().to_lowercase().chars())
            .collect(),
        None => String::new(),
    }
}

fn capitalize(value: Value) -> String {
    capitalize_str(&str_value(&value))
}

/// markupsafe's `escape`: a safe string stays as it is.
fn escape(value: Value) -> Value {
    if value.is_safe() {
        return value;
    }
    let text = str_value(&value)
        .replace('&', "&amp;")
        .replace('>', "&gt;")
        .replace('<', "&lt;")
        .replace('\'', "&#39;")
        .replace('"', "&#34;");
    Value::from_safe_string(text)
}

fn join(value: Value, separator: Option<Value>, kwargs: Kwargs) -> Result<String, Error> {
    let separator = separator.map(|sep| str_value(&sep)).unwrap_or_default();
    let attribute: Option<Value> = kwargs.get("attribute")?;
    kwargs.assert_all_used()?;
    let mut parts = Vec::new();
    for item in py_iter(&value)? {
        let item = match &attribute {
            Some(attribute) => item.get_item(attribute)?,
            None => item,
        };
        parts.push(str_value(&item));
    }
    Ok(parts.join(&separator))
}

fn length(value: Value) -> Result<usize, Error> {
    match value.kind() {
        ValueKind::Undefined => Ok(0),
        ValueKind::String => Ok(value.as_str().unwrap_or_default().chars().count()),
        _ => value
            .len()
            .ok_or_else(|| invalid(format!("object of type '{}' has no len()", value.kind()))),
    }
}

fn list(value: Value) -> Result<Value, Error> {
    Ok(Value::from(py_iter(&value)?))
}

fn lower(value: Value) -> String {
    str_value(&value).to_lowercase()
}

fn upper(value: Value) -> String {
    str_value(&value).to_uppercase()
}

fn replace(value: Value, old: Value, new: Value, count: Option<i64>) -> String {
    let (text, old, new) = (str_value(&value), str_value(&old), str_value(&new));
    match count {
        Some(count) if count >= 0 => text.replacen(&old, &new, count as usize),
        _ => text.replace(&old, &new),
    }
}

fn select(
    state: &mut State,
    value: Value,
    test: Option<Cow<'_, str>>,
    args: Rest<ValueOrKwargs>,
) -> Result<Vec<Value>, Error> {
    match test {
        None => Ok(py_iter(&value)?
            .into_iter()
            .filter(Value::is_true)
            .collect()),
        Some(test) => {
            minijinja::filters::select(state, Value::from(py_iter(&value)?), Some(test), args)
        }
    }
}

fn string(value: Value) -> String {
    str_value(&value)
}

/// Jinja2's `title`: a capital at the start of each word, words split on `-`, spaces and
/// opening brackets.
fn title(value: Value) -> String {
    let text = str_value(&value);
    let is_separator = |ch: char| is_space(ch) || "-({[<".contains(ch);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(&first) = chars.peek() {
        let separator = is_separator(first);
        let mut item = String::new();
        while let Some(&ch) = chars.peek() {
            if is_separator(ch) != separator {
                break;
            }
            item.push(ch);
            chars.next();
        }
        if separator {
            out.push_str(&item);
        } else {
            out.push_str(&capitalize_str(&item));
        }
    }
    out
}

/// Jinja2's `tojson`: `json.dumps(value, sort_keys=True)`, safe in HTML.
fn tojson(value: Value, indent: Option<Value>) -> Result<Value, Error> {
    if indent.is_some_and(|indent| !indent.is_none()) {
        return Err(invalid("tojson with an indent is not supported"));
    }
    let text = json_dumps(&value).map_err(invalid)?;
    Ok(Value::from_safe_string(
        text.replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026")
            .replace('\'', "\\u0027"),
    ))
}

fn trim(value: Value, chars: Option<String>) -> String {
    let text = str_value(&value);
    match chars {
        Some(chars) => text.trim_matches(|ch| chars.contains(ch)).to_string(),
        None => text.trim_matches(is_space).to_string(),
    }
}

/// Python's `==` between two template values: `1 == 1.0 == True`.
fn py_eq(left: &Value, right: &Value) -> bool {
    let number = |value: &Value| match value.kind() {
        ValueKind::Bool => Some(if value.is_true() { 1.0 } else { 0.0 }),
        ValueKind::Number => f64::try_from(value.clone()).ok(),
        _ => None,
    };
    match (number(left), number(right)) {
        (Some(a), Some(b)) => a == b,
        (None, None) => left == right,
        _ => false,
    }
}

/// Jinja2's `unique`: the first of each value, strings compared ignoring case by default.
fn unique(value: Value, kwargs: Kwargs) -> Result<Vec<Value>, Error> {
    let case_sensitive = kwargs
        .get::<Option<bool>>("case_sensitive")?
        .unwrap_or(false);
    let attribute: Option<Value> = kwargs.get("attribute")?;
    kwargs.assert_all_used()?;
    let mut seen: Vec<Value> = Vec::new();
    let mut out = Vec::new();
    for item in py_iter(&value)? {
        let mut key = match &attribute {
            Some(attribute) => item.get_item(attribute)?,
            None => item.clone(),
        };
        if let (false, Some(text)) = (case_sensitive, key.as_str()) {
            key = Value::from(text.to_lowercase());
        }
        if !seen.iter().any(|known| py_eq(known, &key)) {
            seen.push(key);
            out.push(item);
        }
    }
    Ok(out)
}

/// Copier's `pathjoin` global: POSIX paths joined like `PurePosixPath(*parts)`.
pub fn pathjoin(parts: Rest<String>, kwargs: Kwargs) -> Result<String, Error> {
    let mode: Option<String> = kwargs.get("mode")?;
    kwargs.assert_all_used()?;
    if mode.as_deref().is_some_and(|mode| mode != "posix") {
        return Err(invalid("pathjoin: only the posix mode is supported"));
    }
    let mut path = String::new();
    for part in parts.iter() {
        if part.starts_with('/') {
            path = part.clone();
        } else if !part.is_empty() {
            if !path.is_empty() && !path.ends_with('/') {
                path.push('/');
            }
            path.push_str(part);
        }
    }
    Ok(crate::posix::normalize(&path))
}
