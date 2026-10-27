//! The commands of the contract, each a request in and a response out.

mod render;

use serde_json::Value as Json;

use crate::error::{Error, Kind, Result};

pub const COMMANDS: [&str; 1] = ["render"];

/// The response to `input`, a request of `command`.
pub fn run(command: &str, input: &str) -> Result<Json> {
    match command {
        "render" => render::run(input),
        _ => Err(Error::new(
            Kind::Request,
            format!(
                "unknown command {command:?}: slake knows {}",
                COMMANDS.join(", ")
            ),
        )),
    }
}
