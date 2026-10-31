//! `ion render render`: a revision rendered into a new directory, as `copier copy --defaults` does
//! (`ion repo new`, ion-repo's tests).

use std::path::Path;
use std::sync::Arc;

use serde_json::{Map as JsonMap, Value as Json};

use crate::answers::{Answers, Conf, render_context};
use crate::contract::{self, RenderRequest};
use crate::data::Data;
use crate::error::{Error, Kind, Result};
use crate::template::Revision;
use crate::{questions, render, source};

pub fn run(input: &str) -> Result<Json> {
    let request: RenderRequest = contract::parse(input)?;
    let destination = contract::absolute(&request.destination)?;
    let is_new = check_destination(&destination)?;
    let template = request.template.template()?;
    let cache = request.cache.as_deref();
    let revision = Arc::new(source::load(template, cache, request.offline)?);
    let config = revision.config()?;
    let engine = render::engine(&revision);
    let mut answers = Answers {
        init: contract::answers(&request.answers),
        user_defaults: contract::answers(&request.defaults),
        metadata: revision.metadata(),
        ..Answers::default()
    };
    let conf = Conf::new(&revision, &destination);
    questions::ask(&engine, &config, &mut answers, &conf, false)?;
    let context = render_context(&answers, &config, &conf, "render");
    let rendered = render::render(&revision, &engine, &context, &|_| false)?;
    write_new(&rendered, &destination, is_new)?;
    let remembered = answers.to_remember(&config);
    Ok(response(&revision, &remembered, rendered.files.keys()))
}

/// Writes the render; on a failure, Copier removes the directory it created.
fn write_new(rendered: &render::Rendered, destination: &Path, is_new: bool) -> Result<()> {
    let written = render::write(rendered, destination);
    if written.is_err()
        && is_new
        && let Err(cleanup) = std::fs::remove_dir_all(destination)
    {
        eprintln!("ion-render: cannot remove {}: {cleanup}", destination.display());
    }
    written
}

/// Whether the destination is missing; an error when it holds anything.
fn check_destination(destination: &Path) -> Result<bool> {
    if !destination.exists() {
        return Ok(true);
    }
    let not_empty = || {
        Error::new(
            Kind::Request,
            format!("{} is not an empty directory", destination.display()),
        )
    };
    let mut entries = std::fs::read_dir(destination).map_err(|_| not_empty())?;
    if entries.next().is_some() {
        return Err(not_empty());
    }
    Ok(false)
}

fn response<'a>(
    revision: &Revision,
    remembered: &crate::data::Map,
    files: impl Iterator<Item = &'a String>,
) -> Json {
    let mut fields = JsonMap::new();
    if let Some(commit) = &revision.commit {
        fields.insert("commit".into(), commit.as_str().into());
    }
    if let Some(sha) = &revision.sha {
        fields.insert("sha".into(), sha.as_str().into());
    }
    let answers = remembered
        .iter()
        .filter(|(key, _)| !matches!(key, Data::Str(name) if name.starts_with('_')))
        .map(|(key, value)| (key.to_string(), value.to_json()));
    fields.insert("answers".into(), Json::Object(answers.collect()));
    fields.insert(
        "files".into(),
        files.map(|path| Json::from(path.as_str())).collect(),
    );
    contract::success(fields)
}
