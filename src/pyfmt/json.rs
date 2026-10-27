//! Python's `json.dumps()`: ASCII only, `", "` and `": "` separators, keys sorted or kept in
//! their order, an indent or none.

use minijinja::Value;
use minijinja::value::ValueKind;

use super::repr_value;

/// Python's `json.dumps(value, sort_keys=True)`.
pub fn json_dumps(value: &Value) -> Result<String, String> {
    json_dumps_with(value, true, None)
}

/// Python's `json.dumps(value, sort_keys=..., indent=...)`.
pub fn json_dumps_with(
    value: &Value,
    sort_keys: bool,
    indent: Option<usize>,
) -> Result<String, String> {
    let mut writer = JsonWriter {
        sort_keys,
        indent,
        out: String::new(),
    };
    writer.value(value, 0)?;
    Ok(writer.out)
}

struct JsonWriter {
    sort_keys: bool,
    indent: Option<usize>,
    out: String,
}

impl JsonWriter {
    fn value(&mut self, value: &Value, level: usize) -> Result<(), String> {
        match value.kind() {
            ValueKind::None | ValueKind::Undefined => self.out.push_str("null"),
            ValueKind::Bool => self
                .out
                .push_str(if value.is_true() { "true" } else { "false" }),
            ValueKind::Number => {
                let text = repr_value(value);
                self.out.push_str(match text.as_str() {
                    "nan" => "NaN",
                    "inf" => "Infinity",
                    "-inf" => "-Infinity",
                    other => other,
                });
            }
            ValueKind::String => self.string(value.as_str().unwrap_or_default()),
            ValueKind::Seq | ValueKind::Iterable => self.array(value, level)?,
            ValueKind::Map => self.object(value, level)?,
            kind => return Err(format!("Object of type {kind} is not JSON serializable")),
        }
        Ok(())
    }

    fn array(&mut self, value: &Value, level: usize) -> Result<(), String> {
        let items: Vec<Value> = value
            .try_iter()
            .map_err(|error| error.to_string())?
            .collect();
        self.out.push('[');
        for (index, item) in items.iter().enumerate() {
            self.separator(index, level + 1);
            self.value(item, level + 1)?;
        }
        if self.indent.is_some() && !items.is_empty() {
            self.separator(0, level);
        }
        self.out.push(']');
        Ok(())
    }

    fn object(&mut self, value: &Value, level: usize) -> Result<(), String> {
        let mut entries: Vec<(String, Value)> = Vec::new();
        for key in value.try_iter().map_err(|error| error.to_string())? {
            entries.push((key_name(&key)?, value.get_item(&key).unwrap_or_default()));
        }
        if self.sort_keys {
            entries.sort_by(|left, right| left.0.cmp(&right.0));
        }
        self.out.push('{');
        for (index, (key, item)) in entries.iter().enumerate() {
            self.separator(index, level + 1);
            self.string(key);
            self.out.push_str(": ");
            self.value(item, level + 1)?;
        }
        if self.indent.is_some() && !entries.is_empty() {
            self.separator(0, level);
        }
        self.out.push('}');
        Ok(())
    }

    /// Before an item: `, ` on one line, or a comma and a new indented line.
    fn separator(&mut self, index: usize, level: usize) {
        match self.indent {
            None if index > 0 => self.out.push_str(", "),
            None => {}
            Some(width) => {
                if index > 0 {
                    self.out.push(',');
                }
                self.out.push('\n');
                self.out.push_str(&" ".repeat(width * level));
            }
        }
    }

    fn string(&mut self, text: &str) {
        self.out.push('"');
        for ch in text.chars() {
            match ch {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                '\u{8}' => self.out.push_str("\\b"),
                '\u{c}' => self.out.push_str("\\f"),
                ' '..='~' => self.out.push(ch),
                _ => {
                    let mut units = [0u16; 2];
                    for unit in ch.encode_utf16(&mut units) {
                        self.out.push_str(&format!("\\u{unit:04x}"));
                    }
                }
            }
        }
        self.out.push('"');
    }
}

/// A key as `json.dumps` writes it: strings, and numbers, booleans and `None` turned to strings.
fn key_name(key: &Value) -> Result<String, String> {
    Ok(match key.kind() {
        ValueKind::String => key.as_str().unwrap_or_default().to_string(),
        ValueKind::None => "null".into(),
        ValueKind::Bool => if key.is_true() { "true" } else { "false" }.into(),
        ValueKind::Number => repr_value(key),
        kind => {
            return Err(format!(
                "keys must be str, int, float, bool or None, not {kind}"
            ));
        }
    })
}
