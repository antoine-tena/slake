//! Python's methods on template values (`name.split('.')`, `text.count('"')`), with Python's
//! semantics where minijinja-contrib's differ (indices in characters, empty separators).

use minijinja::value::{Value, ValueKind, from_args};
use minijinja::{Error, ErrorKind, State};

pub fn call(
    state: &mut State,
    value: &Value,
    method: &str,
    args: &[Value],
) -> Result<Value, Error> {
    if let Some(text) = value.as_str()
        && let Some(result) = string_method(text, method, args)?
    {
        return Ok(result);
    }
    minijinja_contrib::pycompat::unknown_method_callback(state, value, method, args)
}

/// Python's `str.isspace()`, for one character.
pub fn is_space(ch: char) -> bool {
    ch.is_whitespace() || ('\x1c'..='\x1f').contains(&ch)
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidOperation, message.into())
}

fn char_index(text: &str, byte: usize) -> i64 {
    text[..byte].chars().count() as i64
}

fn string_method(text: &str, method: &str, args: &[Value]) -> Result<Option<Value>, Error> {
    let value = match method {
        "split" | "rsplit" => split(text, args, method == "rsplit")?,
        "count" => count(text, args)?,
        "find" | "index" | "rfind" | "rindex" => find(text, method, args)?,
        "startswith" | "endswith" => has_affix(text, method, args)?,
        "strip" | "lstrip" | "rstrip" => strip(text, method, args)?,
        "replace" => replace(text, args)?,
        "upper" => Value::from(text.to_uppercase()),
        "lower" => Value::from(text.to_lowercase()),
        "capitalize" => Value::from(super::filters::capitalize_str(text)),
        "title" => Value::from(python_title(text)),
        "join" => join(text, args)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn split(text: &str, args: &[Value], from_right: bool) -> Result<Value, Error> {
    let (sep, maxsplit): (Option<Value>, Option<i64>) = from_args(args)?;
    let sep = sep.filter(|sep| !sep.is_none());
    let maxsplit = maxsplit.unwrap_or(-1);
    let parts = match sep.as_ref().and_then(Value::as_str) {
        Some("") => return Err(invalid("empty separator")),
        Some(sep) => split_on(text, sep, maxsplit, from_right),
        None if sep.is_some() => return Err(invalid("must be str or None")),
        None => split_whitespace(text, maxsplit, from_right),
    };
    Ok(Value::from(
        parts.into_iter().map(Value::from).collect::<Vec<_>>(),
    ))
}

fn count(text: &str, args: &[Value]) -> Result<Value, Error> {
    let (sub,): (&str,) = from_args(args)?;
    let count = if sub.is_empty() {
        text.chars().count() + 1
    } else {
        text.matches(sub).count()
    };
    Ok(Value::from(count))
}

fn find(text: &str, method: &str, args: &[Value]) -> Result<Value, Error> {
    let (sub,): (&str,) = from_args(args)?;
    let found = if method.starts_with('r') {
        text.rfind(sub)
    } else {
        text.find(sub)
    };
    match found {
        Some(byte) => Ok(Value::from(char_index(text, byte))),
        None if method.ends_with("index") => Err(invalid("substring not found")),
        None => Ok(Value::from(-1)),
    }
}

/// `startswith` and `endswith`, with one affix or a tuple of them.
fn has_affix(text: &str, method: &str, args: &[Value]) -> Result<Value, Error> {
    let (affix,): (Value,) = from_args(args)?;
    let affixes: Vec<Value> = if affix.as_str().is_some() {
        vec![affix]
    } else if affix.kind() == ValueKind::Seq {
        affix.try_iter()?.collect()
    } else {
        return Err(invalid(format!(
            "{method} first arg must be str or a tuple of str"
        )));
    };
    let mut is_found = false;
    for affix in &affixes {
        let affix = affix
            .as_str()
            .ok_or_else(|| invalid("tuple must contain only strings"))?;
        is_found |= if method == "startswith" {
            text.starts_with(affix)
        } else {
            text.ends_with(affix)
        };
    }
    Ok(Value::from(is_found))
}

fn strip(text: &str, method: &str, args: &[Value]) -> Result<Value, Error> {
    let (chars,): (Option<Value>,) = from_args(args)?;
    let chars: Option<Vec<char>> = chars
        .as_ref()
        .and_then(Value::as_str)
        .map(|chars| chars.chars().collect());
    let strip = |ch: char| match &chars {
        Some(chars) => chars.contains(&ch),
        None => is_space(ch),
    };
    Ok(Value::from(match method {
        "strip" => text.trim_matches(strip),
        "lstrip" => text.trim_start_matches(strip),
        _ => text.trim_end_matches(strip),
    }))
}

fn replace(text: &str, args: &[Value]) -> Result<Value, Error> {
    let (old, new, count): (&str, &str, Option<i64>) = from_args(args)?;
    Ok(Value::from(match count {
        Some(count) if count >= 0 => text.replacen(old, new, count as usize),
        _ => text.replace(old, new),
    }))
}

fn join(text: &str, args: &[Value]) -> Result<Value, Error> {
    let (items,): (Value,) = from_args(args)?;
    let mut parts = Vec::new();
    for item in items.try_iter()? {
        let part = item
            .as_str()
            .ok_or_else(|| invalid("sequence item: expected str instance"))?;
        parts.push(part.to_string());
    }
    Ok(Value::from(parts.join(text)))
}

/// Python's `str.title()`: a capital after any character that is not a letter.
fn python_title(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut previous_cased = false;
    for ch in text.chars() {
        if previous_cased {
            out.extend(ch.to_lowercase());
        } else {
            out.extend(ch.to_uppercase());
        }
        previous_cased = ch.is_alphabetic();
    }
    out
}

fn split_on(text: &str, sep: &str, maxsplit: i64, from_right: bool) -> Vec<String> {
    let limit = if maxsplit < 0 {
        usize::MAX
    } else {
        maxsplit as usize + 1
    };
    if from_right {
        let mut parts: Vec<String> = text.rsplitn(limit, sep).map(String::from).collect();
        parts.reverse();
        parts
    } else {
        text.splitn(limit, sep).map(String::from).collect()
    }
}

fn split_whitespace(text: &str, maxsplit: i64, from_right: bool) -> Vec<String> {
    if from_right {
        let reversed: String = text.chars().rev().collect();
        let mut parts: Vec<String> = split_whitespace(&reversed, maxsplit, false)
            .into_iter()
            .map(|part| part.chars().rev().collect())
            .collect();
        parts.reverse();
        return parts;
    }
    let mut parts = Vec::new();
    let mut rest = text.trim_start_matches(is_space);
    while !rest.is_empty() {
        if maxsplit >= 0 && parts.len() as i64 == maxsplit {
            parts.push(rest.to_string());
            break;
        }
        let end = rest.find(is_space).unwrap_or(rest.len());
        parts.push(rest[..end].to_string());
        rest = rest[end..].trim_start_matches(is_space);
    }
    parts
}
