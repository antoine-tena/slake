//! The Jinja environment Copier renders with, on minijinja: Jinja2's defaults as Copier sets
//! them (trailing newline kept, no autoescape), values written as Python writes them, Jinja2's
//! filters with their Python semantics, and the Ansible filters Copier loads.

mod ansible;
mod filters;
mod methods;
mod rewrite;

use std::sync::Arc;

use minijinja::value::Value;
use minijinja::{AutoEscape, Environment, Error, ErrorKind};

use crate::pyfmt;

pub use methods::is_space;

/// Reads a template file by its name, relative to the template's root.
pub type Loader = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub struct Engine {
    env: Environment<'static>,
}

impl Engine {
    pub fn new(loader: Loader) -> Engine {
        let mut env = Environment::new();
        env.set_syntax(
            minijinja::syntax::SyntaxConfig::builder()
                .keep_trailing_newline(true)
                .build()
                .expect("Jinja2's syntax"),
        );
        env.set_auto_escape_callback(|_| AutoEscape::None);
        env.set_formatter(|out, _state, value| {
            out.write_str(&pyfmt::str_value(value))
                .map_err(|_| Error::new(ErrorKind::WriteFailure, "could not write the output"))
        });
        env.set_unknown_method_callback(methods::call);
        env.set_loader(move |name| match loader(name) {
            Some(source) => rewrite::appends(&source)
                .map(Some)
                .map_err(|message| Error::new(ErrorKind::SyntaxError, message)),
            None => Ok(None),
        });
        filters::register(&mut env);
        env.add_function("pathjoin", filters::pathjoin);
        Engine { env }
    }

    /// Renders a file of the template, as Copier's `get_template(name).render(context)`.
    pub fn render_file(&self, name: &str, context: &Value) -> Result<String, String> {
        let template = self.env.get_template(name).map_err(describe)?;
        template.render(context).map_err(describe)
    }

    /// Renders a string, as Copier's `from_string(source).render(context)`.
    pub fn render_str(&self, source: &str, context: &Value) -> Result<String, String> {
        let source = rewrite::appends(source)?;
        self.env.render_str(&source, context).map_err(describe)
    }
}

fn describe(error: Error) -> String {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        message.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    message
}
