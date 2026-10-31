//! A revision of a template: its files, read from a git repository or from a directory, and the
//! settings and questions of its `copier.yml`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::config::Config;
use crate::data::{Data, Map};
use crate::error::{Error, Kind, Result};
use crate::git;

/// Where the template's files live, under the template's root: Copier's `_subdirectory`, the
/// only one ion-repo uses.
pub const SUBDIRECTORY: &str = "template";
pub const SUFFIX: &str = ".jinja";
pub const ANSWERS_FILE: &str = ".copier-answers.yml";
const SUBDIRECTORY_PREFIX: &str = "template/";

pub struct File {
    pub content: Vec<u8>,
    pub executable: bool,
    /// The exact permissions Copier copies from a template directory outside git; a file read
    /// from git gets the default ones, with the executable bits git records.
    pub mode: Option<u32>,
}

pub struct Revision {
    /// The files a render reads, by their path from the template's root: `copier.yml`, every
    /// file under `template/`, and every `.jinja` file (the macros templates import).
    pub files: BTreeMap<String, File>,
    /// Every directory under `template/`, by its path from the template's root: those of its
    /// files, and the empty ones of a template directory outside git, which Copier creates too.
    pub directories: BTreeSet<String>,
    /// What the answers file records as `_commit`.
    pub commit: Option<String>,
    /// The commit's SHA (`_copier_conf.vcs_ref_hash`).
    pub sha: Option<String>,
    /// What the answers file records as `_src_path`.
    pub src_path: String,
    pub vcs_ref: Option<String>,
}

/// The directories under `template/` that hold the given files.
fn parent_directories<'a>(files: impl Iterator<Item = &'a String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in files.filter(|file| file.starts_with(SUBDIRECTORY_PREFIX)) {
        let mut end = SUBDIRECTORY_PREFIX.len();
        while let Some(slash) = file[end..].find('/') {
            directories.insert(file[..end + slash].to_string());
            end += slash + 1;
        }
    }
    directories
}

fn wanted(path: &str) -> bool {
    path == "copier.yml" || path.starts_with("template/") || path.ends_with(SUFFIX)
}

impl Revision {
    /// A revision of a repository: `reference` is recorded as `_commit`, verbatim.
    pub fn from_git(repository: &Path, sha: &str, url: &str, reference: &str) -> Result<Revision> {
        let files = git_files(repository, sha)?;
        Ok(Revision {
            directories: parent_directories(files.keys()),
            files,
            commit: Some(reference.to_string()),
            sha: Some(sha.to_string()),
            src_path: url.to_string(),
            vcs_ref: Some(reference.to_string()),
        })
    }

    /// A template directory as it is on disk, uncommitted changes included. Copier takes it for a
    /// git checkout only at a repository's root (`.git` a directory): the files git ignores are
    /// then left out. Elsewhere it takes every file, with its exact permissions.
    pub fn from_directory(directory: &Path, commit: Option<String>) -> Result<Revision> {
        if !directory.is_dir() {
            let message = format!("{} is not a directory", directory.display());
            return Err(Error::new(Kind::Request, message));
        }
        let is_git = directory.join(".git").is_dir();
        let (paths, directories) = directory_entries(directory, is_git)?;
        let mut files = BTreeMap::new();
        for path in paths.into_iter().filter(|path| wanted(path)) {
            if let Some(file) = read_file(&directory.join(&path), is_git)? {
                files.insert(path, file);
            }
        }
        Ok(Revision {
            directories: if is_git {
                parent_directories(files.keys())
            } else {
                directories
            },
            files,
            commit,
            sha: if is_git {
                git::resolve(directory, "HEAD")
            } else {
                None
            },
            src_path: directory.to_string_lossy().into_owned(),
            vcs_ref: None,
        })
    }

    /// `_src_path` and `_commit`, as Copier's `Template.metadata`.
    pub fn metadata(&self) -> Map {
        let mut metadata =
            Map::from_iter([(Data::str("_src_path"), Data::str(self.src_path.as_str()))]);
        if let Some(commit) = self.commit.as_deref().filter(|commit| !commit.is_empty()) {
            metadata.insert(Data::str("_commit"), Data::str(commit));
        }
        metadata
    }

    pub fn config(&self) -> Result<Config> {
        let text = self
            .text("copier.yml")
            .ok_or_else(|| Error::template("the template has no copier.yml"))?;
        Config::parse(&text)
    }

    pub fn text(&self, path: &str) -> Option<String> {
        self.files.get(path).map(|file| {
            let text = String::from_utf8_lossy(&file.content);
            // Python reads templates with universal newlines.
            text.replace("\r\n", "\n").replace('\r', "\n")
        })
    }
}

/// The files a render reads in a commit, by one `git cat-file --batch`.
fn git_files(repository: &Path, sha: &str) -> Result<BTreeMap<String, File>> {
    let entries: Vec<_> = git::tree(repository, sha)?
        .into_iter()
        .filter(|entry| wanted(&entry.path))
        .collect();
    if entries
        .iter()
        .any(|entry| entry.symlink && entry.path.starts_with(SUBDIRECTORY_PREFIX))
    {
        return Err(Error::template(
            "symbolic links in template/ are not supported",
        ));
    }
    let objects: Vec<&str> = entries.iter().map(|entry| entry.object.as_str()).collect();
    let contents = git::read_objects(repository, &objects)?;
    let files = entries.into_iter().zip(contents).map(|(entry, content)| {
        let file = File {
            content,
            executable: entry.executable,
            mode: None,
        };
        (entry.path, file)
    });
    Ok(files.collect())
}

/// A file of a template directory; `None` for a path that is no file, such as a deleted file
/// git still lists.
fn read_file(path: &Path, is_git: bool) -> Result<Option<File>> {
    let io = |error: std::io::Error| Error::new(Kind::Io, format!("{}: {error}", path.display()));
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io(error)),
    };
    let permissions = permissions(&metadata);
    Ok(Some(File {
        content: std::fs::read(path).map_err(io)?,
        // git records a file as executable when its owner may run it.
        executable: permissions & 0o100 != 0,
        mode: if is_git { None } else { Some(permissions) },
    }))
}

/// The files of a directory, by their path from it, and outside git the directories under
/// `template/`: what git does not ignore in a checkout, everything elsewhere.
fn directory_entries(directory: &Path, is_git: bool) -> Result<(Vec<String>, BTreeSet<String>)> {
    if is_git {
        let listing = git::git(
            directory,
            &[
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                "--deduplicate",
            ],
        )?;
        let files = listing
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(String::from)
            .collect();
        return Ok((files, BTreeSet::new()));
    }
    let mut files = Vec::new();
    let mut directories = BTreeSet::new();
    walk(directory, directory, &mut files, &mut directories)
        .map_err(|error| Error::new(Kind::Io, format!("{}: {error}", directory.display())))?;
    Ok((files, directories))
}

#[cfg(unix)]
fn permissions(metadata: &std::fs::Metadata) -> u32 {
    std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o7777
}

#[cfg(not(unix))]
fn permissions(metadata: &std::fs::Metadata) -> u32 {
    if metadata.permissions().readonly() {
        0o444
    } else {
        0o666
    }
}

fn walk(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
    directories: &mut BTreeSet<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("under root")
            .to_string_lossy()
            .replace('\\', "/");
        if !path.is_dir() {
            files.push(relative);
        } else if entry.file_name() != ".git" {
            if relative.starts_with(SUBDIRECTORY_PREFIX) {
                directories.insert(relative);
            }
            walk(root, &path, files, directories)?;
        }
    }
    Ok(())
}
