//! The `slake` command: one JSON request on standard input, one JSON response on standard
//! output, logs on standard error (`docs/contract.md`).

use std::io::Read;
use std::process::ExitCode;

use slake::contract;
use slake::error::{Error, Kind};

const USAGE: &str = "usage: slake <command> < request.json";

fn read_request() -> Result<String, Error> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| Error::new(Kind::Request, format!("cannot read the request: {error}")))?;
    Ok(input)
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let response = match arguments.as_slice() {
        [flag] if flag == "--version" => return print(&format!("slake {}", contract::VERSION)),
        [flag] if flag == "--help" => {
            let commands = slake::commands::COMMANDS.join(", ");
            return print(&format!("{USAGE}\ncommands: {commands}"));
        }
        [command] => read_request().and_then(|input| slake::commands::run(command, &input)),
        _ => Err(Error::new(Kind::Request, USAGE)),
    };
    match response {
        Ok(response) => print(&response.to_string()),
        Err(error) => {
            eprintln!("slake: {error}");
            println!("{}", contract::failure(&error));
            ExitCode::FAILURE
        }
    }
}

fn print(text: &str) -> ExitCode {
    println!("{text}");
    ExitCode::SUCCESS
}
