//! YAML as PyYAML reads it: YAML 1.1 types (`yes`, `off`, `0755`, dates) when loading
//! `copier.yml` and answers files.

mod construct;
mod load;
mod resolve;

pub use load::{load, load_all};
