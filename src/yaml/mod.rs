//! YAML as PyYAML reads and writes it: YAML 1.1 types (`yes`, `off`, `0755`, dates) when
//! loading `copier.yml` and answers files, and PyYAML's emitter for the `to_nice_yaml` filter
//! that writes `.copier-answers.yml`.

mod construct;
mod dump;
mod load;
mod resolve;

pub use dump::{DumpOptions, dump};
pub use load::{load, load_all};
