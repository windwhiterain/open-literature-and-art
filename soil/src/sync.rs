use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn read_stripped(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let stripped = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(stripped).into_owned())
}

/// Extract TOML between leading `+++` fences.
fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("+++") {
        let rest = rest.trim_start();
        if let Some(pos) = rest.find("\n+++") {
            return rest[..pos].trim();
        }
        if let Some(pos) = rest.find("\r\n+++") {
            return rest[..pos].trim();
        }
    }
    s
}

/// Extract body — everything after the closing `+++`.
fn strip_frontmatter(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("+++") {
        let rest = rest.trim_start();
        if let Some(pos) = rest.find("\n+++") {
            let after = rest[pos..].trim_start_matches('\n').trim_start_matches("+++");
            return after.trim_start_matches('\n').trim_start_matches("\r\n").trim();
        }
        if let Some(pos) = rest.find("\r\n+++") {
            let after = rest[pos..].trim_start_matches('\n').trim_start_matches("+++");
            return after.trim_start_matches('\n').trim_start_matches("\r\n").trim();
        }
    }
    s
}

/// Inverse of `gen`:
/// - `index.md`       → split into `{dirname}.md` (body) + `meta.toml` (frontmatter)
/// - `_index.md`      → extract frontmatter → `meta.toml`
/// - regular `.md`    → split into `{name}` (body) + `{stem}.meta.toml` (frontmatter)
pub fn run(source: &Path, author: &str, project_root: &Path) -> anyhow::Result<usize> {
    let src_base = project_root.join("content").join(author);
    let mut count = 0usize;

    for entry in WalkDir::new(&src_base).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_str().unwrap_or("");
        let path = entry.path();
        let rel = path.strip_prefix(&src_base)?;
        let parent = source.join(rel.parent().unwrap_or(Path::new("")));
        let content = read_stripped(path)?;

        // --- _index.md → meta.toml (section frontmatter) ---
        if name == "_index.md" {
            // Skip root-level author _index.
            if rel.parent().map_or(true, |p| p.as_os_str().is_empty()) {
                continue;
            }
            let meta = parent.join("meta.toml");
            if !meta.exists() {
                let toml = strip_fences(&content);
                fs::create_dir_all(&parent)?;
                fs::write(&meta, toml)?;
                println!("  + {}", meta.display());
                count += 1;
            }
            continue;
        }

        // --- index.md → {dirname}.md (body) + meta.toml (frontmatter) ---
        if name == "index.md" {
            let dirname = parent.file_name().unwrap().to_str().unwrap_or("untitled");
            let md = parent.join(format!("{}.md", dirname));
            let mt = parent.join("meta.toml");

            let toml = strip_fences(&content);
            let body = strip_frontmatter(&content);

            let need_mt = !mt.exists();
            let need_md = !md.exists();
            if need_mt || need_md {
                fs::create_dir_all(&parent)?;
            }
            if need_mt {
                fs::write(&mt, toml)?;
                println!("  + {}", mt.display());
                count += 1;
            }
            if need_md {
                fs::write(&md, body)?;
                println!("  + {}", md.display());
                count += 1;
            }
            continue;
        }

        // --- regular article .md → {name}.md (body) + {stem}.meta.toml (frontmatter) ---
        if name.ends_with(".md")
            && !name.ends_with(".meta.md")
        {
            let stem = &name[..name.len() - 3];
            let md = parent.join(name);
            let mt = parent.join(format!("{}.meta.toml", stem));

            let toml = strip_fences(&content);
            let body = strip_frontmatter(&content);

            let need_mt = !mt.exists();
            let need_md = !md.exists();
            if need_mt || need_md {
                fs::create_dir_all(&parent)?;
            }
            if need_mt {
                fs::write(&mt, toml)?;
                println!("  + {}", mt.display());
                count += 1;
            }
            if need_md {
                fs::write(&md, body)?;
                println!("  + {}", md.display());
                count += 1;
            }
        }
    }

    println!("Synced {} files to {}", count, source.display());
    Ok(count)
}
