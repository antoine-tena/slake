//! Where a revision is read from: a bare mirror of a remote repository, kept in ion-render's cache
//! and fetched only when needed, a local repository read in place, or a directory.

use std::path::{Path, PathBuf};

use crate::contract::Template;
use crate::error::{Error, Kind, Result};
use crate::git;
use crate::template::Revision;

/// The revision a request names.
pub fn load(template: Template, cache: Option<&Path>, offline: bool) -> Result<Revision> {
    match template {
        Template::Git { url, reference } => {
            let (repository, sha) = match local_repository(&url) {
                Some(repository) => {
                    let sha = git::resolve(&repository, &reference).ok_or_else(|| {
                        Error::new(Kind::Ref, format!("{reference} is not a revision of {url}"))
                    })?;
                    (repository, sha)
                }
                None => {
                    let cache = cache.map_or_else(default_cache, Path::to_path_buf);
                    resolve_in_mirror(&url, &reference, &cache, offline)?
                }
            };
            Revision::from_git(&repository, &sha, &url, &reference)
        }
        Template::Directory { path, commit } => Revision::from_directory(&path, commit),
    }
}

/// A repository on this machine: Copier clones it afresh each time, ion-render reads it in place.
fn local_repository(url: &str) -> Option<PathBuf> {
    let path = Path::new(url.strip_prefix("file://").unwrap_or(url));
    if !path.exists() {
        return None;
    }
    std::path::absolute(path).ok()
}

pub fn default_cache() -> PathBuf {
    match std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        Some(cache) => PathBuf::from(cache).join("ion-render"),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/slake"),
    }
}

/// FNV-1a: a stable name for a mirror, whatever the URL holds.
fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn mirror_path(cache: &Path, url: &str) -> PathBuf {
    let name: String = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("template")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || "-_.".contains(*ch))
        .collect();
    cache
        .join("mirrors")
        .join(format!("{name}-{}.git", fingerprint(url)))
}

fn clone_mirror(url: &str, mirror: &Path, cache: &Path) -> Result<()> {
    std::fs::create_dir_all(mirror.parent().expect("mirror parent"))
        .map_err(|error| Error::new(Kind::Io, format!("cannot create the cache: {error}")))?;
    let partial = mirror.with_extension("partial");
    if partial.exists() {
        std::fs::remove_dir_all(&partial).map_err(|error| {
            Error::new(
                Kind::Io,
                format!("cannot clear {}: {error}", partial.display()),
            )
        })?;
    }
    git::git(
        cache,
        &[
            "clone",
            "--mirror",
            "--quiet",
            url,
            &partial.to_string_lossy(),
        ],
    )
    .map_err(|error| Error::new(Kind::Network, error.message))?;
    std::fs::rename(&partial, mirror).map_err(|error| {
        Error::new(
            Kind::Io,
            format!("cannot move the mirror in place: {error}"),
        )
    })
}

/// The commit `reference` names in the mirror of `url`. The mirror is fetched only when it
/// lacks `reference`, or when `reference` is a branch, which moves.
pub fn resolve_in_mirror(
    url: &str,
    reference: &str,
    cache: &Path,
    offline: bool,
) -> Result<(PathBuf, String)> {
    let (mirror, is_new) = open_mirror(url, cache, offline)?;
    let is_branch =
        reference == "HEAD" || git::resolve(&mirror, &format!("refs/heads/{reference}")).is_some();
    let known = git::resolve(&mirror, reference);
    if let Some(sha) = known.filter(|_| is_new || offline || !is_branch) {
        return Ok((mirror, sha));
    }
    if offline {
        let message = format!("{reference} is not in the mirror of {url}");
        return Err(Error::new(Kind::Ref, message));
    }
    git::git(&mirror, &["fetch", "--quiet", "--prune", "origin"])
        .map_err(|error| Error::new(Kind::Network, error.message))?;
    git::resolve(&mirror, reference)
        .map(|sha| (mirror, sha))
        .ok_or_else(|| Error::new(Kind::Ref, format!("{reference} is not a revision of {url}")))
}

/// The mirror of `url`, cloned when the cache has none; whether it was.
fn open_mirror(url: &str, cache: &Path, offline: bool) -> Result<(PathBuf, bool)> {
    let mirror = mirror_path(cache, url);
    if mirror.join("HEAD").exists() {
        return Ok((mirror, false));
    }
    if offline {
        let message = format!("no mirror of {url} in {} and offline", cache.display());
        return Err(Error::new(Kind::Ref, message));
    }
    clone_mirror(url, &mirror, cache)?;
    Ok((mirror, true))
}
