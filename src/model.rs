use std::path::PathBuf;

/// Top-level site structure
pub struct Site {
    pub authors: Vec<Author>,
}

/// A single author with their articles, collections, and assets
pub struct Author {
    /// Directory name, e.g. "whiting"
    pub name: String,
    /// Rendered README.md if present (not listed as an article)
    pub intro_html: Option<String>,
    /// Standalone articles (top-level .md or single-name-match dir-based)
    pub articles: Vec<Article>,
    /// Collections (directories with multiple .md files or non-matching single .md)
    pub collections: Vec<Collection>,
    /// Static assets (images, non-md files) — not articles
    pub assets: Vec<Asset>,
}

/// A standalone article (one page)
pub struct Article {
    /// Extracted from first H1 or filename stem
    pub title: String,
    /// Source filename, e.g. "高中回忆录.md"
    pub file_name: String,
    /// Output path relative to dist, e.g. "a/whiting/高中回忆录.html"
    pub out_path: String,
    /// Output dir for links resolution, e.g. "a/whiting"
    pub out_dir: String,
    /// Rendered HTML body (without the title H1 — page header already shows it)
    pub html: String,
    /// Publication date (YYYY-MM-DD) from .soil.toml
    pub date: Option<String>,
    /// Whether the article is a draft (skipped during build)
    pub draft: bool,
    /// Short description from .soil.toml
    pub summary: Option<String>,
}

/// A collection (文集) — a directory containing multiple chapters
pub struct Collection {
    /// Directory name, e.g. "红河水"
    pub name: String,
    /// Output dir, e.g. "a/whiting/红河水"
    pub out_dir: String,
    /// Chapters within this collection
    pub chapters: Vec<Chapter>,
    /// Non-image attachments (e.g. .json files)
    pub attachments: Vec<Asset>,
    /// Publication date (YYYY-MM-DD) from .soil.toml
    pub date: Option<String>,
    /// Whether the collection is a draft (skipped during build)
    pub draft: bool,
    /// Short description from .soil.toml
    pub summary: Option<String>,
}

/// A chapter within a collection
pub struct Chapter {
    pub title: String,
    pub file_name: String,
    /// Path relative to collection root, e.g. "" or "subdir/"
    pub rel_dir: String,
    /// Output path, e.g. "a/whiting/红河水/第一章.html"
    pub out_path: String,
    pub html: String,
    /// Publication date (YYYY-MM-DD) from .soil.toml
    pub date: Option<String>,
    /// Whether the chapter is a draft (skipped during build)
    pub draft: bool,
    /// Short description from .soil.toml
    pub summary: Option<String>,
}

/// A non-markdown asset (image, JSON, etc.)
pub struct Asset {
    /// Source absolute path
    pub src: PathBuf,
    /// Output path relative to dist, e.g. "files/whiting/杨朔的愿望/杨朔.png"
    pub out_path: String,
}
