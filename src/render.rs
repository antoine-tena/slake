//! A revision of the template rendered the way Copier renders it: which files, under which
//! names, with which content and mode.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use minijinja::Value;

use crate::error::{Error, Kind, Result};
use crate::jinja::Engine;
use crate::posix::normalize;
use crate::template::{ANSWERS_FILE, File, Revision, SUBDIRECTORY, SUFFIX};

pub struct RenderedFile {
    pub content: Vec<u8>,
    pub executable: bool,
    pub mode: Option<u32>,
}

#[derive(Default)]
pub struct Rendered {
    pub files: BTreeMap<String, RenderedFile>,
    /// Every directory Copier creates, empty ones included.
    pub directories: BTreeSet<String>,
}

/// The engine that reads the revision's files: `macros.jinja` and the templates themselves.
pub fn engine(revision: &Arc<Revision>) -> Engine {
    let revision = Arc::clone(revision);
    Engine::new(Arc::new(move |name: &str| revision.text(name)))
}

/// Renders every entry of `template/`: the files, and the directories Copier creates.
pub fn render(
    revision: &Revision,
    engine: &Engine,
    context: &Value,
    excluded: &dyn Fn(&str) -> bool,
) -> Result<Rendered> {
    let renderer = Renderer {
        revision,
        engine,
        context,
    };
    Ok(Rendered {
        directories: renderer.directories(excluded)?,
        files: renderer.files(excluded)?,
    })
}

/// What a render reads: the revision, the engine on its files, the context.
struct Renderer<'a> {
    revision: &'a Revision,
    engine: &'a Engine,
    context: &'a Value,
}

/// Where the parts of a path render to.
enum Landing {
    /// A part rendered empty: the entry is not rendered.
    Nowhere,
    /// A part rendered to the answers file's name: Copier takes the entry for it.
    AnswersFile,
    Path(String),
}

impl Renderer<'_> {
    /// Every file of `template/` that lands somewhere. Two template files land on one path only
    /// with the same content.
    fn files(&self, excluded: &dyn Fn(&str) -> bool) -> Result<BTreeMap<String, RenderedFile>> {
        let prefix = format!("{SUBDIRECTORY}/");
        let mut files: BTreeMap<String, RenderedFile> = BTreeMap::new();
        for (source, file) in self.revision.files.range(prefix.clone()..) {
            let Some(relative) = source.strip_prefix(&prefix) else {
                break;
            };
            let destination = self.destination(relative)?;
            let Some(path) = destination.filter(|path| !excluded(path)) else {
                continue;
            };
            let rendered = self.rendered_file(source, file)?;
            let existing = files.get(&path);
            if existing.is_some_and(|existing| existing.content != rendered.content) {
                let message = format!("two template files render to {path}");
                return Err(Error::template(message));
            }
            files.insert(path, rendered);
        }
        Ok(files)
    }

    /// Where each directory of `template/` lands, if anywhere.
    fn directories(&self, excluded: &dyn Fn(&str) -> bool) -> Result<BTreeSet<String>> {
        let prefix = format!("{SUBDIRECTORY}/");
        let mut directories = BTreeSet::new();
        let relatives = self.revision.directories.iter();
        for relative in relatives.filter_map(|directory| directory.strip_prefix(&prefix)) {
            if let Some(path) = self.destination(relative)?
                && !excluded(&path)
            {
                directories.insert(path);
            }
        }
        Ok(directories)
    }

    /// Copier's `_render_path`: where a template entry lands, if anywhere. `relative` is its
    /// path under `template/`.
    fn destination(&self, relative: &str) -> Result<Option<String>> {
        let name = relative.rsplit('/').next().unwrap_or(relative);
        let is_template = relative.ends_with(SUFFIX) && name != SUFFIX;
        // A plain file next to its `.jinja` twin is not rendered.
        let twin = format!("{SUBDIRECTORY}/{relative}{SUFFIX}");
        if self.revision.files.contains_key(&twin) {
            return Ok(None);
        }
        let relative = if is_template {
            &relative[..relative.len() - SUFFIX.len()]
        } else {
            relative
        };
        let path = match self.landing(relative)? {
            Landing::Nowhere => return Ok(None),
            Landing::AnswersFile => return Ok(Some(ANSWERS_FILE.to_string())),
            Landing::Path(path) => path,
        };
        // Copier looks for the `.jinja` twin at the template's root here, not under `template/`.
        if !is_template && self.revision.files.contains_key(&format!("{path}{SUFFIX}")) {
            return Ok(None);
        }
        if path.starts_with('/') || path.split('/').any(|part| part == "..") || path == "." {
            let message = format!("{relative} renders outside the project: {path}");
            return Err(Error::template(message));
        }
        Ok(Some(path))
    }

    /// Copier's `_render_parts`: each part of the path rendered on its own.
    fn landing(&self, relative: &str) -> Result<Landing> {
        let mut parts = Vec::new();
        for part in relative.split('/') {
            let rendered = if part.contains('{') {
                self.engine
                    .render_str(part, self.context)
                    .map_err(|error| {
                        Error::template(format!(
                            "Error rendering template path {relative}: {error}"
                        ))
                    })?
            } else {
                part.to_string()
            };
            if rendered.is_empty() {
                return Ok(Landing::Nowhere);
            }
            if rendered == ANSWERS_FILE {
                return Ok(Landing::AnswersFile);
            }
            parts.push(rendered);
        }
        Ok(Landing::Path(normalize(&parts.join("/"))))
    }

    /// A file as it lands: a `.jinja` file rendered, any other copied as it is, its mode kept.
    fn rendered_file(&self, source: &str, file: &File) -> Result<RenderedFile> {
        let content = if source.ends_with(SUFFIX) {
            let text = self.engine.render_file(source, self.context);
            let text = text.map_err(|error| Error::template(format!("{source}: {error}")))?;
            text.into_bytes()
        } else {
            file.content.clone()
        };
        Ok(RenderedFile {
            content,
            executable: file.executable,
            mode: file.mode,
        })
    }
}

/// Writes a render into an empty or missing directory, with the modes Copier gives.
pub fn write(rendered: &Rendered, destination: &Path) -> Result<()> {
    let io = |path: &Path, error: std::io::Error| {
        Error::new(Kind::Io, format!("{}: {error}", path.display()))
    };
    std::fs::create_dir_all(destination).map_err(|error| io(destination, error))?;
    for directory in &rendered.directories {
        let path = destination.join(directory);
        std::fs::create_dir_all(&path).map_err(|error| io(&path, error))?;
    }
    for (relative, file) in &rendered.files {
        let path = destination.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| io(parent, error))?;
        }
        write_file(&path, file).map_err(|error| io(&path, error))?;
    }
    Ok(())
}

/// A new file with the default permissions, then Copier's: the template's exact ones, or the
/// executable bits git records added.
fn write_file(path: &Path, file: &RenderedFile) -> std::io::Result<()> {
    let mut handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    std::io::Write::write_all(&mut handle, &file.content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let current = handle.metadata()?.permissions().mode() & 0o7777;
        let wanted = match file.mode {
            Some(mode) => mode,
            None if file.executable => current | 0o111,
            None => current,
        };
        if wanted != current {
            handle.set_permissions(std::fs::Permissions::from_mode(wanted))?;
        }
    }
    Ok(())
}
