//! Values written the way Python writes them: `str()`, `repr()` and `json.dumps()`.

mod json;

use minijinja::Value;
use minijinja::value::ValueKind;

use crate::data::Data;

pub use json::{json_dumps, json_dumps_with};

/// Python's `repr(float)`: the shortest digits that read back the same, in exponent notation
/// below 1e-4 and from 1e16 on.
pub fn float_repr(value: f64) -> String {
    if value.is_nan() {
        return "nan".into();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.into();
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .into();
    }
    // Rust's `{:e}` gives the same shortest round-trip digits as Python's repr.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("exponent notation");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().expect("exponent");
    let sign = if value < 0.0 { "-" } else { "" };
    let point = exponent + 1;
    if -4 < point && point <= 16 {
        format!("{sign}{}", positional(&digits, point))
    } else {
        format!("{sign}{}", exponential(&digits, exponent))
    }
}

/// Digits written with their point, `point` digits before it.
fn positional(digits: &str, point: i32) -> String {
    if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}.0", "0".repeat(point as usize - digits.len()))
    } else {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{whole}.{fraction}")
    }
}

/// Digits in exponent notation: one before the point, an exponent of two digits at least.
fn exponential(digits: &str, exponent: i32) -> String {
    let (first, rest) = digits.split_at(1);
    let mantissa = if rest.is_empty() {
        first.to_string()
    } else {
        format!("{first}.{rest}")
    };
    let exponent_sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}e{exponent_sign}{:02}", exponent.abs())
}

/// Whether Python's `str.isprintable()` holds for this character, as far as repr needs it.
fn is_printable(ch: char) -> bool {
    let code = ch as u32;
    !(code < 0x20
        || (0x7f..=0xa0).contains(&code)
        || code == 0xad
        || (0x2000..=0x200f).contains(&code)
        || (0x2028..=0x202f).contains(&code)
        || (0x205f..=0x2064).contains(&code)
        || code == 0x1680
        || code == 0x3000
        || code == 0xfeff
        || (0xd800..=0xf8ff).contains(&code)
        || (0xfff0..=0xfffb).contains(&code)
        || code == 0xfffe
        || code == 0xffff)
}

/// Python's `repr(str)`.
pub fn str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ if ch == quote => {
                out.push('\\');
                out.push(ch);
            }
            _ if is_printable(ch) => out.push(ch),
            _ if (ch as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", ch as u32)),
            _ if (ch as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            _ => out.push_str(&format!("\\U{:08x}", ch as u32)),
        }
    }
    out.push(quote);
    out
}

/// Python's `repr()` of loaded data.
pub fn repr_data(data: &Data) -> String {
    match data {
        Data::Null => "None".into(),
        Data::Bool(value) => if *value { "True" } else { "False" }.into(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => float_repr(*value),
        Data::Str(text) => str_repr(text),
        Data::Date(text) => {
            let parts: Vec<&str> = text.split('-').collect();
            let numbers: Vec<String> = parts
                .iter()
                .map(|part| part.trim_start_matches('0').to_string())
                .collect();
            format!("datetime.date({})", numbers.join(", "))
        }
        Data::List(items) => {
            format!(
                "[{}]",
                items.iter().map(repr_data).collect::<Vec<_>>().join(", ")
            )
        }
        Data::Map(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(key, value)| format!("{}: {}", repr_data(key), repr_data(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Python's `str()` of a template value, as Jinja2 writes `{{ value }}`.
pub fn str_value(value: &Value) -> String {
    match value.kind() {
        ValueKind::Undefined => String::new(),
        ValueKind::String => value.as_str().unwrap_or_default().to_string(),
        _ => repr_value(value),
    }
}

/// Python's `repr()` of a template value.
pub fn repr_value(value: &Value) -> String {
    match value.kind() {
        ValueKind::Undefined => String::new(),
        ValueKind::None => "None".into(),
        ValueKind::Bool => if value.is_true() { "True" } else { "False" }.into(),
        ValueKind::Number if value.is_integer() => value.to_string(),
        ValueKind::Number => float_repr(f64::try_from(value.clone()).unwrap_or(f64::NAN)),
        ValueKind::String => str_repr(value.as_str().unwrap_or_default()),
        ValueKind::Seq | ValueKind::Iterable => repr_sequence(value),
        ValueKind::Map => repr_mapping(value),
        _ => value.to_string(),
    }
}

/// A list, or a tuple: `(1,)` for one item.
fn repr_sequence(value: &Value) -> String {
    let items: Vec<String> = value
        .try_iter()
        .map(|iter| iter.map(|item| repr_value(&item)).collect())
        .unwrap_or_default();
    match (value.is_tuple(), items.len()) {
        (true, 1) => format!("({},)", items[0]),
        (true, _) => format!("({})", items.join(", ")),
        (false, _) => format!("[{}]", items.join(", ")),
    }
}

fn repr_mapping(value: &Value) -> String {
    let items: Vec<String> = value
        .try_iter()
        .map(|keys| {
            keys.map(|key| {
                let item = value.get_item(&key).unwrap_or_default();
                format!("{}: {}", repr_value(&key), repr_value(&item))
            })
            .collect()
        })
        .unwrap_or_default();
    format!("{{{}}}", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_read_like_python() {
        let cases = [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (1.5, "1.5"),
            (100.0, "100.0"),
            (1e16, "1e+16"),
            (1.5e16, "1.5e+16"),
            (1e15, "1000000000000000.0"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (-2.5e-7, "-2.5e-07"),
            (123456.789, "123456.789"),
        ];
        for (value, expected) in cases {
            assert_eq!(float_repr(value), expected, "{value}");
        }
    }

    #[test]
    fn strings_quote_like_python() {
        assert_eq!(str_repr("a"), "'a'");
        assert_eq!(str_repr("it's"), "\"it's\"");
        assert_eq!(str_repr("it's \"x\""), "'it\\'s \"x\"'");
        assert_eq!(str_repr("é\n"), "'é\\n'");
    }
}
