//! POSIX paths as Python's `PurePosixPath` reads them.

/// A POSIX path as `PurePosixPath` writes it: no empty or `.` parts, no trailing slash.
pub fn normalize(path: &str) -> String {
    let leading = if path.starts_with("//") && !path.starts_with("///") {
        "//"
    } else if path.starts_with('/') {
        "/"
    } else {
        ""
    };
    let parts: Vec<&str> = path
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    let joined = format!("{leading}{}", parts.join("/"));
    if joined.is_empty() {
        ".".into()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn paths_normalize_like_pure_posix_path() {
        assert_eq!(normalize("a//b/./c/"), "a/b/c");
        assert_eq!(normalize("./x"), "x");
        assert_eq!(normalize("//a"), "//a");
        assert_eq!(normalize("///a"), "/a");
        assert_eq!(normalize(""), ".");
    }
}
