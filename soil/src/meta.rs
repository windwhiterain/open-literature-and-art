use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Scan the current working directory for articles/collections and generate
/// a `<name>.soil.toml` template for each one that doesn't already have one.
/// Existing `.soil.toml` files are NEVER overwritten.
pub fn run() {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("无法获取当前目录: {e}");
            return;
        }
    };
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("扫描文稿目录: {}", canonical.display());

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("无法读取目录: {e}");
            return;
        }
    };

    // Collect children
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    let mut subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().into_owned();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            subdirs.push((name_str, path));
        } else {
            files.push((name_str, path));
        }
    }

    let mut created = 0u32;
    let mut skipped = 0u32;

    // Top-level .md files → standalone articles
    for (fname, fpath) in &files {
        if !fname.ends_with(".md") || fname.eq_ignore_ascii_case("README.md") {
            continue;
        }
        let name = fname.strip_suffix(".md").unwrap_or(fname);
        match generate_meta(&canonical, name, || file_mtime(fpath)) {
            GenerationResult::Created => created += 1,
            GenerationResult::Skipped => skipped += 1,
            GenerationResult::Error(e) => eprintln!("{e}"),
        }
    }

    // Subdirectories → standalone articles or collections
    for (dname, dpath) in &subdirs {
        let (md_files, _) = list_dir_contents(dpath);
        if md_files.is_empty() {
            continue;
        }
        let is_single_match = md_files.len() == 1 && strip_ext(&md_files[0].0) == *dname;
        if is_single_match {
            // Standalone article: date hint = mtime of its single .md file
            match generate_meta(&canonical, dname, || file_mtime(&md_files[0].1)) {
                GenerationResult::Created => created += 1,
                GenerationResult::Skipped => skipped += 1,
                GenerationResult::Error(e) => eprintln!("{e}"),
            }
        } else {
            // Collection: date hint = most recent mtime among all .md files
            match generate_meta(&canonical, dname, || latest_md_mtime(dpath)) {
                GenerationResult::Created => created += 1,
                GenerationResult::Skipped => skipped += 1,
                GenerationResult::Error(e) => eprintln!("{e}"),
            }
        }
    }

    println!();
    if created > 0 {
        println!("{created} 个 .soil.toml 已生成，请编辑其中内容后提交。");
    }
    if skipped > 0 {
        println!("{skipped} 个已存在，跳过（不会覆盖已有文件）。");
    }
    if created == 0 && skipped == 0 {
        println!("未发现任何文章或文集。");
    }
}

enum GenerationResult {
    Created,
    Skipped,
    Error(String),
}

/// Generate `<name>.soil.toml` in `author_dir` only if it does not already
/// exist. `mtime_of` is evaluated lazily, after the existence check.
fn generate_meta(
    author_dir: &Path,
    name: &str,
    mtime_of: impl FnOnce() -> Option<u64>,
) -> GenerationResult {
    let toml_path = author_dir.join(format!("{name}.soil.toml"));
    if toml_path.exists() {
        return GenerationResult::Skipped;
    }
    let date_str = mtime_of().map_or(String::new(), unix_to_date);
    let content = format!("date = \"?~{date_str}\"\ndraft = false\nsummary = \"\"\n");
    match fs::write(&toml_path, &content) {
        Ok(()) => {
            println!(
                "已创建: {}",
                toml_path.file_name().unwrap().to_string_lossy()
            );
            GenerationResult::Created
        }
        Err(e) => GenerationResult::Error(format!("无法写入 {}: {e}", toml_path.display())),
    }
}

/// Most recent modification time among all `.md` files under `dir` (recursive).
fn latest_md_mtime(dir: &Path) -> Option<u64> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".md"))
        .filter_map(|e| file_mtime(&e.into_path()))
        .max()
}

fn file_mtime(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn unix_to_date(secs: u64) -> String {
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert days since 1970-01-01 to (year, month, day) — Howard Hinnant's
/// civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// List the contents of a directory: (.md files, other entries).
fn list_dir_contents(
    dir: &Path,
) -> (
    Vec<(String, std::path::PathBuf)>,
    Vec<(String, std::path::PathBuf)>,
) {
    let mut md = Vec::new();
    let mut other = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return (md, other),
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().into_owned();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if name_str.ends_with(".md") {
            md.push((name_str, path));
        } else {
            other.push((name_str, path));
        }
    }
    (md, other)
}

fn strip_ext(filename: &str) -> &str {
    filename.strip_suffix(".md").unwrap_or(filename)
}
