//! Development probe: one JSON request per line on stdin, one JSON answer per line on stdout,
//! for the scripts that compare slake with Python (PyYAML, Jinja2).

use std::io::{BufRead, Write};

use slake::data::Data;
use slake::yaml;

fn answer(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    match request["op"].as_str() {
        Some("dump") => {
            let value = Data::from_json(&request["value"]).to_value();
            let options = yaml::DumpOptions {
                indent: request["indent"].as_u64().unwrap_or(4) as usize,
                width: request["width"].as_u64().unwrap_or(80) as usize,
                allow_unicode: true,
                default_flow_style: request["flow"].as_bool(),
                sort_keys: true,
            };
            yaml::dump(&value, &options).map(serde_json::Value::String)
        }
        Some("load") => yaml::load(request["text"].as_str().unwrap_or_default())
            .map(|data| serde_json::Value::String(slake::pyfmt::repr_data(&data))),
        Some("render") => {
            let files: std::collections::HashMap<String, String> =
                serde_json::from_value(request["files"].clone()).unwrap_or_default();
            let engine = slake::jinja::Engine::new(std::sync::Arc::new(move |name: &str| {
                files.get(name).cloned()
            }));
            let context = Data::from_json(&request["context"]).to_value();
            engine
                .render_str(request["source"].as_str().unwrap_or_default(), &context)
                .map(serde_json::Value::String)
        }
        _ => Err("unknown op".into()),
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let request: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let reply = match answer(&request) {
            Ok(value) => serde_json::json!({"ok": value}),
            Err(error) => serde_json::json!({"error": error}),
        };
        writeln!(stdout, "{reply}").unwrap();
    }
}
