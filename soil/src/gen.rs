use anyhow::Context;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

fn read_stripped(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)?;
    let stripped = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(stripped).into_owned())
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("+++") {
        let rest = rest.trim_start();
        if let Some(pos) = rest.find("\n+++") { return rest[..pos].trim(); }
        if let Some(pos) = rest.find("\r\n+++") { return rest[..pos].trim(); }
    }
    s
}

fn today_ym() -> String {
    let d = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let days = (d.as_secs() / 86400) as i64;
    let mut y = 1970i64;
    let mut r = days;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if r < dy { break; }
        r -= dy; y += 1;
    }
    let md = if is_leap(y) {
        [31,29,31,30,31,30,31,31,30,31,30,31]
    } else {
        [31,28,31,30,31,30,31,31,30,31,30,31]
    };
    let mut m = 1;
    for d in md { if r < d { break; } r -= d; m += 1; }
    format!("{:04}-{:02}", y, m)
}

fn is_leap(y: i64) -> bool { (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) }

fn dir_has_md(dir: &Path) -> bool {
    WalkDir::new(dir).min_depth(1).max_depth(1).into_iter()
        .any(|e| e.map_or(false, |e| {
            e.file_type().is_file()
                && e.file_name().to_str().map_or(false, |n| n.ends_with(".md") && n != "_index.md")
        }))
}

fn count_md_in(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir).min_depth(1).max_depth(1).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.file_name().to_str().map_or(false, |n| n.ends_with(".md") && n != "_index.md")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn run(source: &Path, author: &str, project_root: &Path) -> anyhow::Result<usize> {
    let dest_base = project_root.join("content").join(author);
    let source_abs = source.canonicalize()
        .with_context(|| format!("Cannot resolve {}", source.display()))?;
    let mut count = 0usize;
    let mut merged: HashSet<PathBuf> = HashSet::new();

    for entry in WalkDir::new(&source_abs).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() { continue; }
        let dir = entry.path();
        if !dir_has_md(dir) { continue; }
        if !dir.join("meta.toml").exists() {
            let title = dir.file_name().unwrap().to_str().unwrap_or("untitled");
            let date = today_ym();
            fs::write(
                dir.join("meta.toml"),
                format!("title = \"{}\"\nsort_by = \"weight\"\n[extra]\ndate = \"{}\"\n", title, date),
            )?;
        }
    }

    // Phase 2: generate section _index.md (or page index.md for single self-named .md)
    for entry in WalkDir::new(&source_abs).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() { continue; }
        let dir = entry.path();
        if !dir.join("meta.toml").exists() { continue; }

        let meta_content = read_stripped(&dir.join("meta.toml"))?;
        let toml = strip_fences(&meta_content);
        let rel = dir.strip_prefix(&source_abs)?;
        let dest_dir = dest_base.join(rel);
        fs::create_dir_all(&dest_dir)?;

        let dirname = dir.file_name().unwrap().to_str().unwrap();
        let self_md = dir.join(format!("{}.md", dirname));
        let md_files = count_md_in(dir);

        if md_files.len() == 1 && self_md.exists() {
            let body = read_stripped(&self_md)?;
            fs::write(
                dest_dir.join("index.md"),
                format!("+++\n{}\ntemplate = \"page.html\"\n+++\n{}", toml, body),
            )?;
            merged.insert(self_md);
            count += 1;
        } else {
            fs::write(
                dest_dir.join("_index.md"),
                format!("+++\n{}\ntemplate = \"work-section.html\"\n+++\n", toml),
            )?;
        }
    }

    for entry in WalkDir::new(&source_abs).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        if merged.contains(path) { continue; }
        let name = path.file_name().unwrap().to_str().unwrap_or("");
        if name.contains(".meta.") || !name.ends_with(".md") || name == "_index.md" { continue; }

        let parent = path.parent().unwrap();
        let stem = &name[..name.len() - 3];
        let meta_path = parent.join(format!("{}.meta.toml", stem));
        if !meta_path.exists() {
            let date = today_ym();
            fs::write(&meta_path, format!("+++\ntitle = \"{}\"\n[extra]\ndate = \"{}\"\n+++\n", stem, date))?;
        }

        let meta_content = read_stripped(&meta_path)?;
        let body_content = read_stripped(path)?;
        let toml = strip_fences(&meta_content);
        let output = format!("+++\n{}\n+++\n{}", toml, body_content);
        let rel = path.strip_prefix(&source_abs)?;
        let rel_dir = rel.parent().unwrap_or(Path::new(""));
        let dest_dir = dest_base.join(rel_dir);
        fs::create_dir_all(&dest_dir)?;
        fs::write(dest_dir.join(name), output)?;
        count += 1;
    }

    let author_index = dest_base.join("_index.md");
    if !author_index.exists() {
        fs::create_dir_all(&dest_base)?;
        fs::write(&author_index, format!(
            "+++\ntitle = \"{}\"\ntemplate = \"author-page.html\"\n+++\n",
            author
        ))?;
    }

    println!("Synced {} articles to {}", count, dest_base.display());
    Ok(count)
}
