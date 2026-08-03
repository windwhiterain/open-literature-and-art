/// Build the output directory path for an author: "a/<name>"
pub fn author_out(name: &str) -> String {
    format!("a/{name}")
}

/// Build the output directory path for static files of an author: "files/<name>"
pub fn files_out(name: &str) -> String {
    format!("files/{name}")
}

/// Convert a filename to a safe output filename (strip .md if present).
/// Full-width characters and UTF-8 are preserved.
pub fn safe_filename(name: &str) -> String {
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

/// Build a relative href from one page to another.
/// `from_depth` = number of path segments from dist root to the current page.
/// `to_path` = path relative to dist root, e.g. "a/whiting/index.html"
pub fn relative_href(from_depth: usize, to_path: &str) -> String {
    let prefix = if from_depth == 0 {
        String::from(".")
    } else {
        "../".repeat(from_depth)
    };
    // Remove leading "./" from to_path
    let clean = to_path.strip_prefix("./").unwrap_or(to_path);
    if prefix.ends_with('/') {
        format!("{prefix}{clean}")
    } else {
        format!("{prefix}/{clean}")
    }
}

/// Get the depth (number of path segments) of an output path from dist root.
/// "index.html" => 0, "a/whiting/index.html" => 2
pub fn depth(out_path: &str) -> usize {
    // Count '/' separators
    out_path.matches('/').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_filename() {
        assert_eq!(safe_filename("test.md"), "test");
        assert_eq!(safe_filename("test"), "test");
        assert_eq!(safe_filename("高中回忆录.md"), "高中回忆录");
    }

    #[test]
    fn test_relative_href() {
        assert_eq!(
            relative_href(0, "a/whiting/index.html"),
            "./a/whiting/index.html"
        );
        assert_eq!(relative_href(2, "index.html"), "../../index.html");
    }

    #[test]
    fn test_depth() {
        assert_eq!(depth("index.html"), 0);
        assert_eq!(depth("a/whiting/index.html"), 2);
        assert_eq!(depth("a/whiting/红河水/第一章.html"), 3);
    }
}
