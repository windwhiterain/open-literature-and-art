use std::fs;
use std::path::{Path, PathBuf};

pub mod content;
pub mod model;
pub mod paths;
pub mod render;
pub mod scan;

/// The about page content, baked into the binary so that soil CLI
/// (which runs sitegen in-process from an arbitrary work_dir) always
/// has a default about page even when `pages/about.md` is absent.
const ABOUT_MD: &str = include_str!("../pages/about.md");

/// The submission guide content, baked in the same way as the about page.
const SUBMIT_MD: &str = include_str!("../pages/submit.md");

pub struct Config {
    pub authors_dir: PathBuf,
    pub out_dir: PathBuf,
    pub base_url: String,
    pub check_links: bool,
}

pub struct BuildSummary {
    pub pages_written: usize,
    pub assets_copied: usize,
    pub warnings: Vec<String>,
}

pub fn run(config: &Config) -> BuildSummary {
    // 1. Scan
    let (site, mut warnings) = scan::scan(&config.authors_dir);

    // 2. Render all HTML pages (random picks happen client-side via JS)
    let mut pages = render::render_site(&site);

    // 2b. Static pages embedded in the binary (same approach as the CSS shell)
    let about_html = content::render_markdown(ABOUT_MD);
    pages.push((
        "about.html".to_string(),
        render::render_about_page(&about_html),
    ));
    let submit_html = content::render_markdown(SUBMIT_MD);
    pages.push((
        "submit.html".to_string(),
        render::render_submit_page(&submit_html),
    ));

    // 4. Write pages to out_dir
    let out = &config.out_dir;
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out).unwrap_or_else(|e| {
        warnings.push(format!("cannot create out dir: {e}"));
    });

    let mut pages_written = 0;
    for (path, content) in &pages {
        let full = out.join(path);
        if let Some(parent) = full.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&full, content).is_ok() {
            pages_written += 1;
        } else {
            warnings.push(format!("cannot write {path}"));
        }
    }

    // 5. Copy static assets (images, etc.)
    let mut assets_copied = 0;
    for author in &site.authors {
        for asset in &author.assets {
            let dest = out.join(&asset.out_path);
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::copy(&asset.src, &dest).is_ok() {
                assets_copied += 1;
            } else {
                warnings.push(format!("cannot copy asset: {}", asset.src.display()));
            }
        }
        for collection in &author.collections {
            for asset in &collection.attachments {
                let dest = out.join(&asset.out_path);
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&asset.src, &dest).is_ok() {
                    assets_copied += 1;
                } else {
                    warnings.push(format!("cannot copy asset: {}", asset.src.display()));
                }
            }
        }
    }

    // 6. Check links if requested
    if config.check_links {
        let link_warnings = check_links_in_dir(out);
        warnings.extend(link_warnings);
    }

    BuildSummary {
        pages_written,
        assets_copied,
        warnings,
    }
}

fn check_links_in_dir(out_dir: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut html_files = Vec::new();

    // Collect all HTML files
    let walker = walkdir::WalkDir::new(out_dir);
    for entry in walker.into_iter().flatten() {
        if entry.path().extension().is_some_and(|e| e == "html") {
            html_files.push(entry.into_path());
        }
    }

    for html_path in &html_files {
        let content = match fs::read_to_string(html_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let parent = html_path.parent().unwrap_or(out_dir);
        warnings.extend(check_html_links(&content, parent, html_path));
    }

    warnings
}

fn check_html_links(html: &str, parent: &Path, source: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    let bytes = html.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        let is_src = bytes[i..].starts_with(b"src=\"");
        let is_href = bytes[i..].starts_with(b"href=\"");

        let (tag_len, attr_start) = if is_src {
            (5, i + 5)
        } else if is_href {
            (6, i + 6)
        } else {
            i += 1;
            continue;
        };

        let mut j = attr_start;
        while j < bytes.len() && bytes[j] != b'"' {
            j += 1;
        }

        if j >= bytes.len() {
            break;
        }

        let url = &html[attr_start..j];

        // Skip external URLs and anchors
        if !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("mailto:")
            && !url.starts_with("#")
            && !url.starts_with("data:")
            && !url.is_empty()
        {
            // Resolve relative URL against the page's directory, then normalize
            let joined = parent.join(url);
            let resolved = normalize_path(&joined);

            if !resolved.exists() {
                // Try appending index.html for directory-style links (trailing slash)
                if url.ends_with('/') {
                    let index_path = resolved.join("index.html");
                    if !index_path.exists() {
                        warnings.push(format!(
                            "broken link in {}: {}",
                            source
                                .strip_prefix(parent.parent().unwrap_or(parent))
                                .unwrap_or(source)
                                .display(),
                            url
                        ));
                    }
                } else {
                    warnings.push(format!(
                        "broken link in {}: {}",
                        source
                            .strip_prefix(parent.parent().unwrap_or(parent))
                            .unwrap_or(source)
                            .display(),
                        url
                    ));
                }
            }
        }

        i = j + 1;
        let _ = tag_len;
    }

    warnings
}

/// Normalize a path by resolving `..` and `.` components without accessing the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    let mut result = PathBuf::new();
    for c in components {
        result.push(c);
    }
    result
}
