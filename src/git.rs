//! git through its command line: one `cat-file --batch` process reads every file of a revision.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Kind};

pub fn git(directory: &Path, args: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .map_err(|error| Error::new(Kind::Io, format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(Error::new(
            Kind::Io,
            format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The commit a revision names, or `None` when the repository does not have it.
pub fn resolve(directory: &Path, revision: &str) -> Option<String> {
    let spec = format!("{revision}^{{commit}}");
    git(
        directory,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &spec,
        ],
    )
    .ok()
    .map(|sha| sha.trim().to_string())
    .filter(|sha| !sha.is_empty())
}

pub struct TreeEntry {
    pub path: String,
    pub object: String,
    pub executable: bool,
    pub symlink: bool,
}

/// Every file of a commit, recursively.
pub fn tree(directory: &Path, sha: &str) -> Result<Vec<TreeEntry>, Error> {
    let listing = git(directory, &["ls-tree", "-r", "-z", "--full-tree", sha])?;
    let mut entries = Vec::new();
    for record in listing.split('\0').filter(|record| !record.is_empty()) {
        let (meta, path) = record
            .split_once('\t')
            .ok_or_else(|| Error::new(Kind::Io, "bad git ls-tree output"))?;
        let fields: Vec<&str> = meta.split(' ').collect();
        if fields.len() != 3 || fields[1] != "blob" {
            // Submodules (commits) are not part of a Copier render.
            continue;
        }
        entries.push(TreeEntry {
            path: path.to_string(),
            object: fields[2].to_string(),
            executable: fields[0] == "100755",
            symlink: fields[0] == "120000",
        });
    }
    Ok(entries)
}

/// The content of many objects, read by one `git cat-file --batch`.
pub fn read_objects(directory: &Path, objects: &[&str]) -> Result<Vec<Vec<u8>>, Error> {
    let io = |error: std::io::Error| Error::new(Kind::Io, format!("git cat-file: {error}"));
    let mut child = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(io)?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let requests: String = objects.iter().map(|object| format!("{object}\n")).collect();
    let writer = std::thread::spawn(move || stdin.write_all(requests.as_bytes()));
    let mut reader = BufReader::new(child.stdout.take().expect("piped stdout"));
    let contents = objects
        .iter()
        .map(|object| read_object(&mut reader, object))
        .collect::<Result<Vec<_>, _>>()?;
    writer.join().expect("writer thread").map_err(io)?;
    child.wait().map_err(io)?;
    Ok(contents)
}

/// One object of `git cat-file --batch`'s output: its header, then its content and a newline.
fn read_object(reader: &mut impl BufRead, object: &str) -> Result<Vec<u8>, Error> {
    let io = |error: std::io::Error| Error::new(Kind::Io, format!("git cat-file: {error}"));
    let mut header = String::new();
    reader.read_line(&mut header).map_err(io)?;
    let size: usize = header
        .split(' ')
        .nth(2)
        .and_then(|size| size.trim().parse().ok())
        .ok_or_else(|| {
            let message = format!("git cat-file cannot read {object}: {}", header.trim());
            Error::new(Kind::Io, message)
        })?;
    let mut content = vec![0; size + 1];
    reader.read_exact(&mut content).map_err(io)?;
    content.pop();
    Ok(content)
}
