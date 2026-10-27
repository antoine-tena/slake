//! Python's casts of an answer: `bool()` the way Copier reads a boolean, `float()`, `int()`.

use crate::data::Data;

/// Python's `bool(float(value))` first, then YAML's words for true and false.
pub fn cast_to_bool(value: &Data) -> bool {
    match value {
        Data::Bool(value) => *value,
        Data::Int(value) => *value != 0,
        Data::Float(value) => *value != 0.0,
        Data::Null => false,
        Data::Str(text) => {
            if let Some(number) = py_float(text) {
                return number != 0.0;
            }
            let lower = text.trim().to_lowercase();
            if lower.is_empty() {
                false
            } else if ["y", "yes", "t", "true", "on"].contains(&lower.as_str()) {
                true
            } else {
                !["n", "no", "f", "false", "off", "~", "null", "none"].contains(&lower.as_str())
            }
        }
        Data::Date(_) => true,
        Data::List(items) => !items.is_empty(),
        Data::Map(map) => !map.is_empty(),
    }
}

/// Python's `float(text)`.
pub fn py_float(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('_')
        || trimmed.ends_with('_')
        || trimmed.contains("__")
    {
        return None;
    }
    let cleaned = trimmed.replace('_', "");
    let lower = cleaned.to_lowercase();
    let unsigned = lower.trim_start_matches(['+', '-']);
    if ["inf", "infinity", "nan"].contains(&unsigned)
        || unsigned
            .chars()
            .all(|ch| ch.is_ascii_digit() || ".e+-".contains(ch))
    {
        cleaned.parse().ok()
    } else {
        None
    }
}

/// Python's `int(text)`.
pub fn py_int(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed.starts_with('_') || trimmed.ends_with('_') || trimmed.contains("__") {
        return None;
    }
    let cleaned = trimmed.replace('_', "");
    let digits = cleaned.trim_start_matches(['+', '-']);
    if digits.is_empty()
        || !digits.chars().all(|ch| ch.is_ascii_digit())
        || cleaned.matches(['+', '-']).count() > 1
    {
        return None;
    }
    cleaned.parse().ok()
}
