//! PyYAML's `yaml.dump`, ported: representer, serializer and emitter, for what templates hand
//! to `to_nice_yaml` (`None`, booleans, numbers, strings, lists, tuples, dicts).

mod analysis;
mod emitter;
mod scalars;
mod writer;

use minijinja::Value;
use minijinja::value::ValueKind;

use crate::pyfmt::float_repr;

pub struct DumpOptions {
    pub indent: usize,
    pub width: usize,
    pub allow_unicode: bool,
    /// `None` puts a collection of plain scalars in flow style, as PyYAML does.
    pub default_flow_style: Option<bool>,
    pub sort_keys: bool,
}

enum Node {
    Scalar {
        tag: String,
        value: String,
    },
    Seq {
        tag: String,
        items: Vec<Node>,
        flow: bool,
    },
    Map {
        tag: String,
        pairs: Vec<(Node, Node)>,
        flow: bool,
    },
}

const STR: &str = "tag:yaml.org,2002:str";
const SEQ: &str = "tag:yaml.org,2002:seq";
const MAP: &str = "tag:yaml.org,2002:map";
const TUPLE: &str = "tag:yaml.org,2002:python/tuple";

/// `yaml.dump(value, **options)`.
pub fn dump(value: &Value, options: &DumpOptions) -> Result<String, String> {
    let node = represent(value, options)?;
    emitter::emit(&node, options)
}

fn scalar(tag: &str, value: String) -> Node {
    Node::Scalar {
        tag: tag.into(),
        value,
    }
}

fn represent(value: &Value, options: &DumpOptions) -> Result<Node, String> {
    Ok(match value.kind() {
        ValueKind::None => scalar("tag:yaml.org,2002:null", "null".into()),
        ValueKind::Bool => scalar(
            "tag:yaml.org,2002:bool",
            if value.is_true() { "true" } else { "false" }.into(),
        ),
        ValueKind::Number if value.is_integer() => {
            scalar("tag:yaml.org,2002:int", value.to_string())
        }
        ValueKind::Number => {
            let number = f64::try_from(value.clone()).map_err(|error| error.to_string())?;
            scalar("tag:yaml.org,2002:float", represent_float(number))
        }
        ValueKind::String => scalar(STR, value.as_str().unwrap_or_default().to_string()),
        ValueKind::Seq | ValueKind::Iterable => represent_sequence(value, options)?,
        ValueKind::Map => represent_mapping(value, options)?,
        kind => return Err(format!("cannot represent an object of kind {kind}")),
    })
}

/// PyYAML's `represent_float`: `repr()`, with `.0` before an exponent that has no point.
fn represent_float(number: f64) -> String {
    if number.is_nan() {
        return ".nan".into();
    }
    if number.is_infinite() {
        return if number > 0.0 { ".inf" } else { "-.inf" }.into();
    }
    let text = float_repr(number).to_lowercase();
    if !text.contains('.') && text.contains('e') {
        text.replacen('e', ".0e", 1)
    } else {
        text
    }
}

fn represent_sequence(value: &Value, options: &DumpOptions) -> Result<Node, String> {
    let items = value
        .try_iter()
        .map_err(|error| error.to_string())?
        .map(|item| represent(&item, options))
        .collect::<Result<Vec<_>, _>>()?;
    let flow = flow_style(options, items.iter().all(is_plain_scalar));
    let tag = if value.is_tuple() { TUPLE } else { SEQ };
    Ok(Node::Seq {
        tag: tag.into(),
        items,
        flow,
    })
}

fn represent_mapping(value: &Value, options: &DumpOptions) -> Result<Node, String> {
    let mut keys: Vec<Value> = value
        .try_iter()
        .map_err(|error| error.to_string())?
        .collect();
    if options.sort_keys {
        sort_keys(&mut keys);
    }
    let mut pairs = Vec::new();
    for key in keys {
        let item = value.get_item(&key).map_err(|error| error.to_string())?;
        pairs.push((represent(&key, options)?, represent(&item, options)?));
    }
    let best = pairs
        .iter()
        .all(|(key, item)| is_plain_scalar(key) && is_plain_scalar(item));
    Ok(Node::Map {
        tag: MAP.into(),
        pairs,
        flow: flow_style(options, best),
    })
}

// Every scalar the representer makes has no style: it counts for the flow style.
fn is_plain_scalar(node: &Node) -> bool {
    matches!(node, Node::Scalar { .. })
}

fn flow_style(options: &DumpOptions, best: bool) -> bool {
    options.default_flow_style.unwrap_or(best)
}

/// Python's `sorted()` on the keys; mixed types it cannot compare stay in their order.
fn sort_keys(keys: &mut [Value]) {
    if keys.iter().all(|key| key.kind() == ValueKind::String) {
        keys.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    } else if keys
        .iter()
        .all(|key| matches!(key.kind(), ValueKind::Number | ValueKind::Bool))
    {
        let number = |key: &Value| f64::try_from(key.clone()).unwrap_or(f64::NAN);
        keys.sort_by(|a, b| {
            number(a)
                .partial_cmp(&number(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

/// The tag as the emitter would write it (`!!str`): it counts toward a simple key's length.
fn prepare_tag(tag: &str) -> String {
    match tag.strip_prefix("tag:yaml.org,2002:") {
        Some(suffix) if !suffix.is_empty() => format!("!!{suffix}"),
        _ => tag.to_string(),
    }
}
