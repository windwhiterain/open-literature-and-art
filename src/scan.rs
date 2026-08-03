use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Article, Asset, Author, Chapter, Collection, Site};
use crate::paths;

/// Scan the `authors_dir` and build a complete `Site` structure.
/// Returns warnings for skipped files.
pub fn scan(authors_dir: &Path) -> (Site, Vec<String>) {
    let mut warnings = Vec::new();
    let mut authors = Vec::new();

    let entries = match fs::read_dir(authors_dir) {
        Ok(e) => e,
        Err(e) => {
            warnings.push(format!("cannot read authors dir: {e}"));
            return (Site { authors }, warnings);
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().into_owned();
        if name_str.starts_with('.') {
            continue;
        }

        let path = entry.path();
        if !path.is_dir() {
            warnings.push(format!("stray file in authors/: {name_str}"));
            continue;
        }

        authors.push(scan_author(&path, &name_str, &mut warnings));
    }

    // Sort authors by name (lexicographic)
    authors.sort_by(|a, b| a.name.cmp(&b.name));

    (Site { authors }, warnings)
}

fn scan_author(dir: &Path, name: &str, warnings: &mut Vec<String>) -> Author {
    let mut intro_html = None;
    let mut articles = Vec::new();
    let mut collections = Vec::new();
    let mut assets = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warnings.push(format!("cannot read author dir {}: {e}", name));
            return Author {
                name: name.to_string(),
                intro_html: None,
                articles,
                collections,
                assets,
            };
        }
    };

    // Collect all entries first so we can classify subdirectories
    let mut child_files: Vec<fs::DirEntry> = Vec::new();
    let mut child_dirs: Vec<fs::DirEntry> = Vec::new();

    for entry in entries.flatten() {
        let n = entry.file_name();
        let ns = n.to_string_lossy();
        if ns.starts_with('.') {
            continue;
        }
        let p = entry.path();
        if p.is_dir() {
            child_dirs.push(entry);
        } else {
            child_files.push(entry);
        }
    }

    let author_out = paths::author_out(name);
    let files_dir = paths::files_out(name);

    // Process files
    for entry in &child_files {
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy().into_owned();
        let path = entry.path();

        // README.md is author intro
        if fname_str.eq_ignore_ascii_case("README.md") {
            intro_html = Some(read_and_render(&path, dir, name, 2, warnings));
            continue;
        }

        if fname_str.ends_with(".md") {
            // Top-level .md = standalone article
            let html = read_and_render(&path, dir, name, 2, warnings);
            let title = crate::content::extract_title(&html, &fname_str);
            let out_name = paths::safe_filename(&fname_str);
            articles.push(Article {
                title,
                file_name: fname_str,
                out_path: format!("{author_out}/{out_name}.html"),
                out_dir: author_out.clone(),
                html: crate::content::strip_title_html(&html),
            });
        } else {
            // Non-md file = asset
            assets.push(Asset {
                out_path: format!("{files_dir}/{fname_str}"),
                src: path,
            });
        }
    }

    // Process subdirectories
    for entry in &child_dirs {
        let dname = entry.file_name();
        let dname_str = dname.to_string_lossy().into_owned();
        let dpath = entry.path();

        let (articles_from_dir, chapters, dir_assets, is_collection, ws) =
            scan_dir_as_content(&dpath, &dname_str, name, &author_out, &files_dir, warnings);

        warnings.extend(ws);

        if is_collection {
            // Natural-sort chapters
            let mut sorted = chapters;
            sorted.sort_by(|a, b| a.title.cmp(&b.title));
            collections.push(Collection {
                name: dname_str.clone(),
                out_dir: format!("{author_out}/{dname_str}"),
                chapters: sorted,
                attachments: dir_assets,
            });
        } else {
            // Standalone article from directory: push the single article + any assets
            articles.extend(articles_from_dir);
            assets.extend(dir_assets);
        }
    }

    // Natural-sort articles by title
    articles.sort_by(|a, b| a.title.cmp(&b.title));

    Author {
        name: name.to_string(),
        intro_html,
        articles,
        collections,
        assets,
    }
}

/// Scan a subdirectory of an author and determine whether it's a Collection or
/// a standalone Article. Returns:
/// - articles_from_dir: standalone article if not collection
/// - chapters: chapters if collection
/// - dir_assets: non-md assets found in the directory
/// - is_collection: true if collection, false if standalone article
#[allow(clippy::too_many_arguments)]
fn scan_dir_as_content(
    dir: &Path,
    dir_name: &str,
    _author_name: &str,
    author_out: &str,
    files_dir_base: &str,
    parent_warnings: &mut Vec<String>,
) -> (Vec<Article>, Vec<Chapter>, Vec<Asset>, bool, Vec<String>) {
    let mut warnings = Vec::new();
    let mut md_entries: Vec<(String, PathBuf)> = Vec::new(); // (filename, path)
    let mut other_entries: Vec<(String, PathBuf, bool)> = Vec::new(); // (name, path, is_dir)

    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let n = entry.file_name();
                let ns = n.to_string_lossy().into_owned();
                if ns.starts_with('.') {
                    continue;
                }
                let p = entry.path();
                if p.is_dir() {
                    other_entries.push((ns, p, true));
                } else if ns.ends_with(".md") {
                    md_entries.push((ns, p));
                } else {
                    other_entries.push((ns, p, false));
                }
            }
        }
        Err(e) => {
            warnings.push(format!("cannot read dir {dir_name}: {e}"));
            return (Vec::new(), Vec::new(), Vec::new(), true, warnings);
        }
    }

    // Classification: standalone article IF exactly 1 .md AND its stem == dir_name
    let is_single_match = md_entries.len() == 1 && strip_ext(&md_entries[0].0) == dir_name;

    let files_dir = format!("{files_dir_base}/{dir_name}");

    if is_single_match {
        // Standalone article from directory
        let (fname, fpath) = md_entries.into_iter().next().unwrap();
        let depth = 3; // a/<author>/<dirname>/index.html
        let html = read_and_render(&fpath, dir, &files_dir, depth, parent_warnings);
        let title = crate::content::extract_title(&html, &fname);
        let article = Article {
            title,
            file_name: fname,
            out_path: format!("{author_out}/{dir_name}/index.html"),
            out_dir: format!("{author_out}/{dir_name}"),
            html: crate::content::strip_title_html(&html),
        };

        // Collect assets from other entries
        let mut assets = Vec::new();
        for (name, path, _is_dir) in other_entries {
            assets.push(Asset {
                out_path: format!("{files_dir}/{name}"),
                src: path,
            });
        }

        (vec![article], Vec::new(), assets, false, warnings)
    } else {
        // Collection
        let mut chapters = Vec::new();
        let mut attachments = Vec::new();

        let col_out = format!("{author_out}/{dir_name}");

        for (fname, fpath) in md_entries {
            let depth = 3; // a/<author>/<collection>/<chapter>.html
            let html = read_and_render(&fpath, dir, &files_dir, depth, parent_warnings);
            let title = crate::content::extract_title(&html, &fname);
            let out_name = paths::safe_filename(&fname);
            chapters.push(Chapter {
                title,
                file_name: fname,
                rel_dir: String::new(),
                out_path: format!("{col_out}/{out_name}.html"),
                html: crate::content::strip_title_html(&html),
            });
        }

        for (name, path, is_dir) in other_entries {
            if is_dir {
                // Recurse into nested directories as more chapters/assets
                let (_, nested_chapters, nested_assets, _, ws) = scan_dir_as_content(
                    &path,
                    &name,
                    _author_name,
                    &col_out,
                    &files_dir,
                    parent_warnings,
                );
                warnings.extend(ws);
                chapters.extend(nested_chapters);
                attachments.extend(nested_assets);
            } else {
                attachments.push(Asset {
                    out_path: format!("{files_dir}/{name}"),
                    src: path,
                });
            }
        }

        (Vec::new(), chapters, attachments, true, warnings)
    }
}

fn strip_ext(filename: &str) -> &str {
    filename.strip_suffix(".md").unwrap_or(filename)
}

fn read_and_render(
    path: &Path,
    src_dir: &Path,
    files_prefix: &str,
    depth: usize,
    warnings: &mut Vec<String>,
) -> String {
    match fs::read_to_string(path) {
        Ok(content) => {
            let html = crate::content::render_markdown(&content);
            let (rewritten, link_warnings) =
                crate::content::rewrite_links(&html, src_dir, depth, files_prefix);
            warnings.extend(link_warnings);
            rewritten
        }
        Err(e) => {
            warnings.push(format!("cannot read {}: {e}", path.display()));
            String::new()
        }
    }
}
