use pulldown_cmark::{Options, Parser};

/// Render markdown to HTML (full document — includes the H1 title)
pub fn render_markdown(input: &str) -> String {
    // Strip a leading UTF-8 BOM — Windows editors (Notepad, PowerShell)
    // write one, and it would otherwise make the first `#` line fail to
    // parse as a heading.
    let input = input.trim_start_matches('\u{feff}');

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(input, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// Extract the title from rendered HTML.
/// Returns the text of the first H1, or falls back to the filename stem.
pub fn extract_title(html: &str, filename: &str) -> String {
    // Extract text from the first <h1>...</h1> in the rendered HTML.
    // pulldown-cmark renders H1 as <h1>text</h1>
    if let Some(start) = html.find("<h1>") {
        let inner_start = start + 4; // after <h1>
        if let Some(end) = html[inner_start..].find("</h1>") {
            let inner = &html[inner_start..inner_start + end];
            // Strip any nested HTML tags (e.g. <em>, <strong>) from the title
            let text = strip_html_tags(inner).trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }
    }

    // Fallback: use filename without .md extension
    filename.strip_suffix(".md").unwrap_or(filename).to_string()
}

/// Remove HTML tags from a string, keeping the text content.
fn strip_html_tags(input: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// Strip the first H1 from rendered HTML so the page template can show the title
/// in its own header without duplication.
pub fn strip_title_html(html: &str) -> String {
    // Simple approach: find the first <h1>...</h1> and remove it, along with
    // surrounding whitespace/newlines.
    if let Some(start) = html.find("<h1>")
        && let Some(end) = html[start..].find("</h1>")
    {
        let after = &html[start + end + 5..];
        let before = &html[..start];
        let before_trimmed = before.trim_end();
        let after_trimmed = after.trim_start();
        return format!("{before_trimmed}\n{after_trimmed}")
            .trim()
            .to_string();
    }
    html.to_string()
}

/// Rewrite local image/link URLs in rendered HTML to point to `files/` output paths.
/// `src_dir` is the original directory of the .md file (for resolving relative paths).
/// `page_depth` is how many `../` to prepend to reach dist root from this page.
/// `files_prefix` is the `files/<author>` prefix for this author.
pub fn rewrite_links(
    html: &str,
    src_dir: &std::path::Path,
    page_depth: usize,
    files_prefix: &str,
) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Look for src=" or href="
        let is_src = bytes[i..].starts_with(b"src=\"");
        let is_href = bytes[i..].starts_with(b"href=\"");

        let (tag_len, attr_start) = if is_src {
            (5, i + 5)
        } else if is_href {
            (6, i + 6)
        } else {
            // Copy one UTF-8 character, not one byte
            let ch = html[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
            continue;
        };

        // Find the closing quote
        let mut j = attr_start;
        while j < bytes.len() && bytes[j] != b'"' {
            j += 1;
        }

        if j >= bytes.len() {
            result.push_str(&html[i..]);
            break;
        }

        let url = &html[attr_start..j];
        let rewritten = rewrite_single_url(url, src_dir, page_depth, files_prefix, &mut warnings);

        if let Some(new_url) = rewritten {
            let prefix = if is_src { "src=\"" } else { "href=\"" };
            result.push_str(prefix);
            result.push_str(&new_url);
            result.push('"');
        } else {
            result.push_str(&html[i..=j]);
        }

        i = j + 1;
        let _ = tag_len;
    }

    (result, warnings)
}

fn rewrite_single_url(
    url: &str,
    src_dir: &std::path::Path,
    page_depth: usize,
    files_prefix: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    // Don't rewrite absolute URLs or anchors
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("#")
        || url.starts_with("data:")
    {
        return None;
    }

    // Resolve the relative URL against the source directory
    let resolved = src_dir.join(url);
    if !resolved.exists() {
        warnings.push(format!("broken link in {}: {url}", src_dir.display()));
        return None;
    }

    // Compute the output path: strip the authors/ prefix
    let src_str = src_dir.to_string_lossy();
    // Find "authors/<name>/" in the path
    let author_prefix = src_str
        .find("authors/")
        .map(|pos| &src_str[pos + "authors/".len()..])
        .unwrap_or(&src_str);

    let relative_to_author = resolved.to_string_lossy().replace('\\', "/");
    let idx = relative_to_author.find(author_prefix).unwrap_or(0);
    let rel = &relative_to_author[idx + author_prefix.len()..];
    let rel = rel.trim_start_matches('/');

    let prefix = "../".repeat(page_depth);
    Some(format!("{prefix}{files_prefix}/{rel}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_with_h1() {
        let md = "# Hello World\n\nSome content.";
        let html = render_markdown(md);
        let title = extract_title(&html, "fallback.md");
        assert_eq!(title, "Hello World");
    }

    #[test]
    fn test_extract_title_fallback() {
        let md = "No heading here\nJust content.";
        let html = render_markdown(md);
        let title = extract_title(&html, "my-article.md");
        assert_eq!(title, "my-article");
    }

    #[test]
    fn test_render_markdown_strips_bom() {
        // Notepad/PowerShell add a UTF-8 BOM; the first heading must still parse.
        let md = "\u{feff}# Hello World\n\nSome content.";
        let html = render_markdown(md);
        assert!(
            html.contains("<h1>Hello World</h1>"),
            "BOM broke the heading: {html}"
        );
        let title = extract_title(&html, "fallback.md");
        assert_eq!(title, "Hello World");
    }

    #[test]
    fn test_strip_title_html() {
        let md = "# Title\n\nBody text.";
        let html = render_markdown(md);
        let stripped = strip_title_html(&html);
        assert!(!stripped.contains("<h1>"));
        assert!(stripped.contains("Body text"));
    }

    #[test]
    fn test_strip_title_html_no_h1() {
        let md = "Just body text.";
        let html = render_markdown(md);
        let stripped = strip_title_html(&html);
        assert!(stripped.contains("body text"));
    }
}
