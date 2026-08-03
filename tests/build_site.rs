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

    fs::write(
        alice.join("README.md"),
        "# About Alice\n\nAlice is a writer.\n",
    )
    .unwrap();
    fs::write(alice.join("随笔.md"), "# 随笔\n\nSome content here.\n").unwrap();

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

    let story = alice.join("杨朔的愿望");
    fs::create_dir_all(&story).unwrap();
    fs::write(
        story.join("杨朔的愿望.md"),
        "# 杨朔的愿望\n\nThis is a story.\n",
    )
    .unwrap();
    fs::write(story.join("photo.png"), b"fake-png-data").unwrap();

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

    // Authors page
    let authors_path = out_dir.join("authors.html");
    assert!(authors_path.exists(), "authors.html missing");
    let authors_html = fs::read_to_string(&authors_path).unwrap();
    assert!(authors_html.contains("换一批"));
    assert!(
        authors_html.contains("a/alice/index.html"),
        "author data missing"
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

    // Dir-based standalone article
    assert!(out_dir.join("a/alice/杨朔的愿望/index.html").exists());
    let story_html = fs::read_to_string(out_dir.join("a/alice/杨朔的愿望/index.html")).unwrap();
    assert!(story_html.contains("杨朔的愿望"));
    assert!(story_html.contains("author-line"));

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

    // Non-matching single md → collection
    assert!(out_dir.join("a/alice/大纲/index.html").exists());

    // Assets copied
    assert!(out_dir.join("files/alice/杨朔的愿望/photo.png").exists());

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
