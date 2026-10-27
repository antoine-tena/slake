//! The Ansible filters Copier loads (`jinja2_ansible_filters`): those kiln's templates use, with
//! Python's semantics.

use std::sync::LazyLock;

use chrono::{Local, TimeZone};
use minijinja::value::{Kwargs, Rest, Value, ValueKind};
use minijinja::{Environment, Error};

use super::filters::invalid;
use crate::pyfmt::str_value;
use crate::yaml::{DumpOptions, dump};

pub fn register(env: &mut Environment<'static>) {
    env.add_filter("bool", to_bool);
    env.add_filter("regex_search", regex_search);
    env.add_filter("regex_replace", regex_replace);
    env.add_filter("strftime", strftime);
    env.add_filter("to_json", to_json);
    env.add_filter("to_nice_json", to_nice_json);
    env.add_filter("to_yaml", to_yaml);
    env.add_filter("to_nice_yaml", to_nice_yaml);
}

/// Ansible's `bool`.
fn to_bool(value: Value) -> bool {
    match value.kind() {
        ValueKind::Bool => value.is_true(),
        ValueKind::None | ValueKind::Undefined => false,
        ValueKind::String => {
            matches!(
                value.as_str().unwrap_or_default().to_lowercase().as_str(),
                "yes" | "on" | "1" | "true"
            )
        }
        _ => value.as_i64() == Some(1),
    }
}

static END_ANCHOR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\\.)|(\[(?:\\.|[^\]])*\])|(\$)").expect("anchor regex"));

/// A Python `re` pattern for fancy-regex: Python's `$` also matches before a final newline.
fn python_regex(
    pattern: &str,
    ignorecase: bool,
    multiline: bool,
) -> Result<fancy_regex::Regex, Error> {
    let translated = if multiline {
        pattern.to_string()
    } else {
        python_end_anchors(pattern)
    };
    let mut flags = String::new();
    if ignorecase {
        flags.push('i');
    }
    if multiline {
        flags.push('m');
    }
    let translated = if flags.is_empty() {
        translated
    } else {
        format!("(?{flags}){translated}")
    };
    fancy_regex::Regex::new(&translated)
        .map_err(|error| invalid(format!("bad regex {pattern:?}: {error}")))
}

/// `$` outside a character class, and `\\Z`, as Python reads them without `re.MULTILINE`.
fn python_end_anchors(pattern: &str) -> String {
    END_ANCHOR
        .replace_all(pattern, |found: &regex::Captures| {
            if found.get(3).is_some() {
                r"(?=\n?\z)".to_string()
            } else {
                found[0].to_string()
            }
        })
        .replace(r"\Z", r"\z")
}

fn regex_flags(kwargs: &Kwargs) -> Result<(bool, bool), Error> {
    Ok((
        kwargs.get::<Option<bool>>("ignorecase")?.unwrap_or(false),
        kwargs.get::<Option<bool>>("multiline")?.unwrap_or(false),
    ))
}

/// Ansible's `regex_search`: the match, the groups asked for (`'\\1'`, `'\\g<name>'`), or `None`.
fn regex_search(
    value: Value,
    pattern: String,
    args: Rest<Value>,
    kwargs: Kwargs,
) -> Result<Value, Error> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid("expected string or bytes-like object"))?;
    let (ignorecase, multiline) = regex_flags(&kwargs)?;
    let regex = python_regex(&pattern, ignorecase, multiline)?;
    let Some(found) = regex
        .captures(text)
        .map_err(|error| invalid(error.to_string()))?
    else {
        return Ok(Value::from(()));
    };
    if args.is_empty() {
        return Ok(Value::from(found.get(0).map_or("", |m| m.as_str())));
    }
    let groups = args
        .iter()
        .map(|arg| group(&found, arg.as_str().unwrap_or_default()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::from(groups))
}

/// The group a `regex_search` argument names: `'\\1'` by number, `'\\g<name>'` by name.
fn group(found: &fancy_regex::Captures<'_, str>, reference: &str) -> Result<Value, Error> {
    let matched = if let Some(name) = reference
        .strip_prefix("\\g<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        found.name(name)
    } else if let Some(index) = reference
        .strip_prefix('\\')
        .and_then(|digits| digits.parse::<usize>().ok())
    {
        found.get(index)
    } else {
        return Err(invalid("Unknown argument"));
    };
    Ok(matched.map_or(Value::from(()), |m| Value::from(m.as_str())))
}

/// Ansible's `regex_replace`: Python's `re.sub`, its `\\1` references included.
fn regex_replace(
    value: Value,
    pattern: String,
    replacement: String,
    kwargs: Kwargs,
) -> Result<String, Error> {
    let text = str_value(&value);
    let (ignorecase, multiline) = regex_flags(&kwargs)?;
    let regex = python_regex(&pattern, ignorecase, multiline)?;
    let replacement = regex::Regex::new(r"\\(\d+)|\\g<(\w+)>")
        .expect("reference regex")
        .replace_all(&replacement, |found: &regex::Captures| {
            format!(
                "${{{}}}",
                found.get(1).or(found.get(2)).map_or("", |m| m.as_str())
            )
        })
        .into_owned();
    Ok(regex.replace_all(&text, replacement.as_str()).into_owned())
}

/// Ansible's `strftime`: `time.strftime(format, time.localtime(second))`.
fn strftime(format: String, second: Option<Value>) -> Result<String, Error> {
    let moment = match second.filter(|second| !second.is_none()) {
        None => Local::now(),
        Some(second) => {
            let seconds = second
                .as_i64()
                .or_else(|| second.as_str().and_then(|text| text.trim().parse().ok()))
                .ok_or_else(|| invalid(format!("Invalid value for epoch value ({second})")))?;
            Local
                .timestamp_opt(seconds, 0)
                .single()
                .ok_or_else(|| invalid("epoch value out of range"))?
        }
    };
    let items: Vec<_> = chrono::format::StrftimeItems::new(&format).collect();
    if items
        .iter()
        .any(|item| matches!(item, chrono::format::Item::Error))
    {
        return Err(invalid(format!("unsupported strftime format {format:?}")));
    }
    Ok(moment.format_with_items(items.into_iter()).to_string())
}

/// The `json.dumps` arguments the JSON filters take: `sort_keys` and `indent`.
fn json_options(
    kwargs: &Kwargs,
    sort_keys: bool,
    indent: Option<usize>,
) -> Result<(bool, Option<usize>), Error> {
    let sort_keys = kwargs
        .get::<Option<bool>>("sort_keys")?
        .unwrap_or(sort_keys);
    let indent = if kwargs.has("indent") {
        kwargs.get::<Option<usize>>("indent")?
    } else {
        indent
    };
    kwargs.assert_all_used()?;
    Ok((sort_keys, indent))
}

/// Ansible's `to_json`: `json.dumps(value, **kwargs)`, keys in their order by default.
fn to_json(value: Value, kwargs: Kwargs) -> Result<String, Error> {
    let (sort_keys, indent) = json_options(&kwargs, false, None)?;
    crate::pyfmt::json_dumps_with(&value, sort_keys, indent).map_err(invalid)
}

/// Ansible's `to_nice_json`: `json.dumps(value, indent=4, sort_keys=True)`.
fn to_nice_json(value: Value, indent: Option<usize>, kwargs: Kwargs) -> Result<String, Error> {
    let (sort_keys, named_indent) = json_options(&kwargs, true, Some(4))?;
    crate::pyfmt::json_dumps_with(&value, sort_keys, indent.or(named_indent)).map_err(invalid)
}

fn yaml_options(kwargs: &Kwargs, indent: usize, flow: Option<bool>) -> Result<DumpOptions, Error> {
    let options = DumpOptions {
        indent: kwargs.get::<Option<usize>>("indent")?.unwrap_or(indent),
        width: kwargs.get::<Option<usize>>("width")?.unwrap_or(80),
        allow_unicode: kwargs.get::<Option<bool>>("allow_unicode")?.unwrap_or(true),
        default_flow_style: if kwargs.has("default_flow_style") {
            kwargs.get::<Option<bool>>("default_flow_style")?
        } else {
            flow
        },
        sort_keys: kwargs.get::<Option<bool>>("sort_keys")?.unwrap_or(true),
    };
    kwargs.assert_all_used()?;
    Ok(options)
}

/// Ansible's `to_yaml`: `yaml.dump(value, allow_unicode=True, default_flow_style=None)`.
fn to_yaml(value: Value, kwargs: Kwargs) -> Result<String, Error> {
    dump(&value, &yaml_options(&kwargs, 2, None)?).map_err(invalid)
}

/// Ansible's `to_nice_yaml`: `yaml.dump(value, indent=4, allow_unicode=True,
/// default_flow_style=False)`.
fn to_nice_yaml(value: Value, indent: Option<usize>, kwargs: Kwargs) -> Result<String, Error> {
    let mut options = yaml_options(&kwargs, 4, Some(false))?;
    if let Some(indent) = indent {
        options.indent = indent;
    }
    dump(&value, &options).map_err(invalid)
}
