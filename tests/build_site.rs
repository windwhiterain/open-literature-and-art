use std::fs;
use std::path::Path;

use sitegen::{Config, run};

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("sitegen-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup_fixture(tmp: &Path) {
    let authors = tmp.join("authors");
    let alice = authors.join("alice");
    fs::create_dir_all(&alice).unwrap();

    // Author repo link comes from the repo-root .gitmodules
    fs::write(
        tmp.join(".gitmodules"),
        "[submodule \"authors/alice\"]\n\tpath = authors/alice\n\turl = https://github.com/example/alice.git\n",
    )
    .unwrap();

    fs::write(
        alice.join("README.md"),
        "# About Alice\n\nAlice is a writer.\n",
    )
    .unwrap();
    fs::write(alice.join("随笔.md"), "# 随笔\n\nSome content here.\n").unwrap();
    fs::write(
        alice.join("随笔.soil.toml"),
        "date = \"2024-03-15\"\nsummary = \"一篇随笔\"\n",
    )
    .unwrap();

    // Draft article: has .soil.toml with draft = true → skipped from build
    fs::write(alice.join("草稿.md"), "# 草稿\n\n未完成。\n").unwrap();
    fs::write(alice.join("草稿.soil.toml"), "draft = true\n").unwrap();

    let wenji = alice.join("文集");
    fs::create_dir_all(&wenji).unwrap();
    fs::write(
        wenji.join("第一章·开始.md"),
        "# 第一章：开始\n\n故事开始了。\n",
    )
    .unwrap();
    fs::write(
        wenji.join("第二章·发展.md"),
        "# 第二章：发展\n\n故事继续。\n",
    )
    .unwrap();
    fs::write(wenji.join("设定.md"), "# 设定\n\n世界观设定。\n").unwrap();
    // Collection-level metadata + one chapter-level metadata file
    fs::write(
        alice.join("文集.soil.toml"),
        "date = \"2024-01-01\"\nsummary = \"一个文集\"\n",
    )
    .unwrap();
    fs::write(
        wenji.join("第一章·开始.soil.toml"),
        "date = \"2024-01-10\"\nsummary = \"开篇章节\"\n",
    )
    .unwrap();

    let story = alice.join("杨朔的愿望");
    fs::create_dir_all(&story).unwrap();
    fs::write(
        story.join("杨朔的愿望.md"),
        "# 杨朔的愿望\n\nThis is a story.\n",
    )
    .unwrap();
    fs::write(story.join("photo.png"), b"fake-png-data").unwrap();
    // Auto-detected date with "?~" prefix: parser must strip it
    fs::write(
        alice.join("杨朔的愿望.soil.toml"),
        "date = \"?~2023-06-01\"\nsummary = \"一个故事\"\n",
    )
    .unwrap();

    let outline = alice.join("大纲");
    fs::create_dir_all(&outline).unwrap();
    fs::write(outline.join("大纲.md"), "# 大纲\n\n大纲内容。\n").unwrap();
}

#[test]
fn test_full_build() {
    let tmp = TempDir::new();
    setup_fixture(tmp.path());

    let out_dir = tmp.path().join("dist");

    let config = Config {
        authors_dir: tmp.path().join("authors"),
        out_dir: out_dir.clone(),
        base_url: String::new(),
        check_links: true,
    };

    let summary = run(&config);

    eprintln!("warnings: {:?}", summary.warnings);
    assert!(summary.pages_written > 0, "no pages written");
    assert!(summary.assets_copied > 0, "no assets copied");

    // Articles page (homepage): nav + JS items + shuffle button
    let index_path = out_dir.join("index.html");
    assert!(index_path.exists(), "index.html missing");
    let index_html = fs::read_to_string(&index_path).unwrap();
    assert!(index_html.contains("换一批"), "shuffle button missing");
    assert!(index_html.contains("id=\"card-list\""));
    assert!(
        index_html.contains("a/alice/随笔.html"),
        "article data missing"
    );
    assert!(index_html.contains("a/alice/杨朔的愿望/index.html"));
    // Top-level collection appears as a single mixed entry
    assert!(
        index_html.contains("a/alice/文集/index.html"),
        "collection entry missing"
    );
    assert!(
        index_html.contains("a/alice/大纲/index.html"),
        "collection entry missing"
    );
    // Chapters must NOT appear in the homepage list
    assert!(
        !index_html.contains("第一章：开始"),
        "chapter must not appear in homepage list"
    );
    assert!(index_html.contains("authors.html"), "authors nav missing");
    assert!(
        index_html.contains("class=\"active\""),
        "active nav missing"
    );
    // Metadata in homepage JS items
    assert!(
        index_html.contains("d:\"2024-03-15\""),
        "article date missing from homepage data"
    );
    assert!(
        index_html.contains("s:\"一篇随笔\""),
        "article summary missing from homepage data"
    );
    // Homepage cards carry the author name + author page link
    assert!(
        index_html.contains("au:\"alice\""),
        "author name missing from homepage data"
    );
    assert!(
        index_html.contains("ah:\"a/alice/index.html\""),
        "author href missing from homepage data"
    );
    // Draft article excluded from homepage
    assert!(
        !index_html.contains("草稿"),
        "draft article must not appear on homepage"
    );

    // Authors page
    let authors_path = out_dir.join("authors.html");
    assert!(authors_path.exists(), "authors.html missing");
    let authors_html = fs::read_to_string(&authors_path).unwrap();
    assert!(authors_html.contains("换一批"));
    assert!(
        authors_html.contains("a/alice/index.html"),
        "author data missing"
    );

    // About page: nav link on homepage + the page exists with key content
    assert!(
        index_html.contains("about.html"),
        "about nav link missing from homepage"
    );
    let about_path = out_dir.join("about.html");
    assert!(about_path.exists(), "about.html missing");
    let about_html = fs::read_to_string(&about_path).unwrap();
    assert!(
        about_html.contains("关于本站"),
        "about page content missing"
    );
    assert!(
        about_html.contains("class=\"active\""),
        "about page nav active state missing"
    );

    // Author page
    let author_index = out_dir.join("a/alice/index.html");
    assert!(author_index.exists(), "author page missing");
    let author_html = fs::read_to_string(&author_index).unwrap();
    assert!(author_html.contains("About Alice"));
    assert!(
        author_html.contains("topnav"),
        "top nav missing on author page"
    );
    // Author git repo link from .gitmodules (trailing .git stripped)
    assert!(
        author_html.contains("Git 仓库"),
        "repo link missing on author page"
    );
    assert!(
        author_html.contains("https://github.com/example/alice\""),
        "repo URL missing on author page"
    );

    // Author page mixes articles and collections in one sorted list
    assert!(
        !author_html.contains("<h2>文章</h2>"),
        "articles/collections must be mixed, not separate sections"
    );
    assert!(!author_html.contains("<h2>文集</h2>"));
    let list_start = author_html.find("<ul class=\"article-list\">").unwrap();
    let list_end = author_html.find("</ul>").unwrap();
    let list_html = &author_html[list_start..list_end];
    assert!(
        list_html.contains("./随笔.html"),
        "article missing from list"
    );
    assert!(
        list_html.contains("./杨朔的愿望/index.html"),
        "dir-based article missing from list"
    );
    assert!(
        list_html.contains("./文集/index.html"),
        "collection missing from list"
    );
    assert!(
        list_html.contains("./大纲/index.html"),
        "collection missing from list"
    );
    // Lexicographic order: 大纲 < 文集 < 杨朔的愿望 < 随笔
    assert!(
        list_html.find("大纲").unwrap() < list_html.find("文集").unwrap(),
        "list must be sorted lexicographically"
    );
    // Metadata rendered in author page list
    assert!(
        list_html.contains("class=\"list-date\">2024-03-15"),
        "date missing from author list"
    );
    assert!(
        list_html.contains("class=\"list-summary\">一篇随笔"),
        "summary missing from author list"
    );
    assert!(
        !author_html.contains("草稿"),
        "draft article must not appear on author page"
    );
    // Author page list shows no per-item author names (homepage cards only)
    assert!(
        !list_html.contains("list-author"),
        "author page list must not show author names"
    );

    // Standalone article: author link at top, NO bottom links
    assert!(out_dir.join("a/alice/随笔.html").exists());
    let article_html = fs::read_to_string(out_dir.join("a/alice/随笔.html")).unwrap();
    assert!(article_html.contains("author-line"), "author link missing");
    assert!(
        article_html.contains("a/alice/index.html"),
        "author href missing"
    );
    assert!(
        !article_html.contains("class=\"back-link\""),
        "standalone article must not have bottom links"
    );
    // Article metadata rendered
    assert!(
        article_html.contains("<meta name=\"description\" content=\"一篇随笔\">"),
        "description meta missing"
    );
    assert!(
        article_html.contains("class=\"article-meta\">2024-03-15"),
        "article date missing"
    );
    assert!(
        article_html.contains("class=\"summary\">一篇随笔"),
        "article summary missing"
    );

    // Draft article: not built at all
    assert!(
        !out_dir.join("a/alice/草稿.html").exists(),
        "draft article must not be built"
    );

    // Dir-based standalone article with "?~" auto-detected date
    assert!(out_dir.join("a/alice/杨朔的愿望/index.html").exists());
    let story_html = fs::read_to_string(out_dir.join("a/alice/杨朔的愿望/index.html")).unwrap();
    assert!(story_html.contains("杨朔的愿望"));
    assert!(story_html.contains("author-line"));
    assert!(
        story_html.contains("class=\"article-meta\">2023-06-01"),
        "?~ prefix not stripped from date"
    );
    assert!(
        !story_html.contains("?~2023"),
        "auto-detected ?~ marker must not leak into HTML"
    );
    assert!(
        story_html.contains("class=\"summary\">一个故事"),
        "dir-based article summary missing"
    );

    // Collection: author link at top, NO bottom links
    let col_index = out_dir.join("a/alice/文集/index.html");
    assert!(col_index.exists(), "collection index missing");
    let col_html = fs::read_to_string(&col_index).unwrap();
    assert!(col_html.contains("第一章：开始"));
    assert!(col_html.contains("第二章：发展"));
    assert!(col_html.contains("author-line"), "author link missing");
    assert!(
        !col_html.contains("class=\"back-link\""),
        "collection page must not have bottom links"
    );
    // Collection-level metadata rendered
    assert!(
        col_html.contains("<meta name=\"description\" content=\"一个文集\">"),
        "collection description meta missing"
    );
    assert!(
        col_html.contains("class=\"article-meta\">2024-01-01"),
        "collection date missing"
    );
    assert!(
        col_html.contains("class=\"summary\">一个文集"),
        "collection summary missing"
    );

    // Chapter page: author link at top, prev/next at bottom
    let chapter_path = out_dir.join("a/alice/文集/第一章·开始.html");
    assert!(chapter_path.exists(), "chapter page missing");
    let chapter_html = fs::read_to_string(&chapter_path).unwrap();
    assert!(chapter_html.contains("author-line"));
    assert!(
        chapter_html.contains("chapter-nav"),
        "prev/next nav missing"
    );
    // First chapter: no prev link, has next link
    assert!(
        !chapter_html.contains("上一章"),
        "first chapter must not have prev"
    );
    assert!(chapter_html.contains("下一章 →"));
    assert!(
        !chapter_html.contains("class=\"back-link\""),
        "chapter page must not have back links"
    );
    // Chapter-level metadata rendered
    assert!(
        chapter_html.contains("class=\"article-meta\">2024-01-10"),
        "chapter date missing"
    );
    assert!(
        chapter_html.contains("class=\"summary\">开篇章节"),
        "chapter summary missing"
    );

    // Non-matching single md → collection
    assert!(out_dir.join("a/alice/大纲/index.html").exists());

    // Assets copied
    assert!(out_dir.join("files/alice/杨朔的愿望/photo.png").exists());
    // .soil.toml files are metadata, never copied as assets
    assert!(
        !out_dir.join("files/alice/随笔.soil.toml").exists(),
        ".soil.toml must not be copied as an asset"
    );

    // --check-links: no broken links
    let warnings: Vec<&str> = summary
        .warnings
        .iter()
        .filter(|w| w.contains("broken link"))
        .map(|s| s.as_str())
        .collect();
    assert!(warnings.is_empty(), "broken links: {:?}", warnings);

    eprintln!(
        "pages: {}, assets: {}",
        summary.pages_written, summary.assets_copied
    );
}
