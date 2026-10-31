//! The `ion render` command: one JSON request on standard input, one JSON response on standard
//! output, logs on standard error (`docs/contract.md`).

use std::io::Read;
use std::process::ExitCode;

use ion_render::contract;
use ion_render::error::{Error, Kind};

const USAGE: &str = "usage: ion-render <command> < request.json";

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
        [flag] if flag == "--version" => return print(&format!("ion-render {}", contract::VERSION)),
        [flag] if flag == "--help" => {
            let commands = ion_render::commands::COMMANDS.join(", ");
            return print(&format!("{USAGE}\ncommands: {commands}"));
        }
        [command] => read_request().and_then(|input| ion_render::commands::run(command, &input)),
        _ => Err(Error::new(Kind::Request, USAGE)),
    };
    match response {
        Ok(response) => print(&response.to_string()),
        Err(error) => {
            eprintln!("ion-render: {error}");
            println!("{}", contract::failure(&error));
            ExitCode::FAILURE
        }
    }
}

fn print(text: &str) -> ExitCode {
    println!("{text}");
    ExitCode::SUCCESS
}
