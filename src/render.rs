use crate::model::{Article, Author, Chapter, Collection, Site};
use crate::paths;

const CSS: &str = r#"*{box-sizing:border-box;margin:0;padding:0}body{font-family:"PingFang SC","Microsoft YaHei","Noto Sans CJK SC","SimHei",sans-serif;line-height:1.9;color:#222;background:#fff;max-width:42rem;margin:0 auto;padding:1.5rem 1rem;font-size:18px}
.topbar{display:flex;justify-content:space-between;align-items:baseline;padding-bottom:.5rem;border-bottom:1px solid #eee;margin-bottom:1.5rem}
.topbar .site-name{font-size:1.15rem;font-weight:700;color:#222;text-decoration:none}
.topnav a{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#555;text-decoration:none;margin-left:1rem;font-size:15px}
.topnav a:hover{color:#000}
.topnav a.active{color:#222;font-weight:600;border-bottom:2px solid #222}
.breadcrumb{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-size:15px;color:#666;margin-bottom:2rem;padding-bottom:.5rem;border-bottom:1px solid #eee}
.breadcrumb a{color:#555;text-decoration:none}
.breadcrumb a:hover{color:#000;text-decoration:underline}
h1{font-size:2rem;text-align:center;margin:2rem 0 .5rem;font-weight:700}
.subtitle{text-align:center;color:#888;font-size:15px;margin-bottom:1.5rem}
h2{font-size:1.4rem;margin:2rem 0 .8rem;font-weight:600}
h3{font-size:1.15rem;margin:1.5rem 0 .5rem}
p{margin:.8em 0}
a{color:#1a5fbf;text-decoration:none}
a:hover{text-decoration:underline}
em{font-style:normal}
ul,ol{padding-left:1.5rem;margin:.8em 0}
li{margin:.3em 0}
blockquote{border-left:3px solid #ddd;margin:1em 0;padding:.3em 1em;color:#555}
pre,code{font-family:"Cascadia Code","Fira Code",monospace;font-size:.9em;background:#f5f5f5;border-radius:4px}
code{padding:.1em .3em}
pre{padding:1em;overflow-x:auto;margin:1em 0}
pre code{padding:0;background:none}
hr{border:none;border-top:1px solid #eee;margin:2rem 0}
img{max-width:100%;height:auto;border-radius:4px}
.author-intro{margin-bottom:2rem;padding:1rem;background:#fafafa;border-radius:6px;font-size:16px}
.article-list{list-style:none;padding:0}
.article-list li{margin:.7rem 0}
.article-list .card{display:block;position:relative;padding:.85rem 1rem;background:#fafafa;border:1px solid #eee;border-radius:8px;color:#222;text-decoration:none;transition:background .15s,border-color .15s}
.article-list .card:hover{background:#f2f2f2;border-color:#ddd;text-decoration:none}
.article-list a.card:focus-visible{outline:2px solid #1a5fbf;outline-offset:2px}
.article-list .card:has(.stretched:focus-visible){outline:2px solid #1a5fbf;outline-offset:2px}
.card .stretched{color:inherit;text-decoration:none}
.card .stretched:focus-visible{outline:none}
.card .stretched::after{content:'';position:absolute;inset:0}
.card-title{font-size:1.1rem}
.list-date{font-size:13px;color:#999;margin-left:.5em}
.list-author{font-size:13px;margin-left:.8em}
.list-author a{color:#555;position:relative;z-index:1}
.list-summary{font-size:14px;color:#666;margin:.2em 0 0}
.empty-state{color:#999;text-align:center;padding:2rem}
.author-line{text-align:center;color:#888;font-size:15px;margin:0 0 1.5rem}
.author-line a{color:#555}
.repo-line{text-align:center;color:#888;font-size:15px;margin:0 0 1.5rem}
.repo-line a{color:#555}
.article-meta{text-align:center;color:#999;font-size:14px;margin:.3rem 0 1rem}
.summary{color:#555;border-left:3px solid #eee;padding:.3em 1em;margin:0 0 1.5em;font-size:16px}
.chapter-nav{display:flex;justify-content:space-between;margin-top:2.5rem;font-size:15px}
.chapter-nav a{color:#555}
.chapter-nav .placeholder{visibility:hidden}
.attachments{font-size:15px;color:#555;margin-top:1rem;padding:1rem;background:#fafafa;border-radius:6px}
.attachments ul{list-style:none;padding:0}
.shuffle-bar{text-align:center;margin:2.5rem 0 1rem}
#shuffle-btn{font-family:inherit;font-size:1rem;padding:.45rem 1.6rem;border:1px solid #ccc;border-radius:20px;background:#fff;color:#333;cursor:pointer}
#shuffle-btn:hover{background:#f5f5f5;border-color:#999}
"#;

/// Generate all pages and return a list of (path, content) pairs
pub fn render_site(site: &Site) -> Vec<(String, String)> {
    let mut pages = Vec::new();

    // Articles page (homepage)
    let articles = collect_articles(site);
    pages.push(("index.html".to_string(), render_articles_page(&articles)));

    // Authors page
    let authors = collect_authors(site);
    pages.push(("authors.html".to_string(), render_authors_page(&authors)));

    // Author pages + their articles + collections
    for author in &site.authors {
        let author_out = paths::author_out(&author.name);
        pages.push((
            format!("{author_out}/index.html"),
            render_author_page(author),
        ));

        // Standalone article pages
        for article in &author.articles {
            pages.push((
                article.out_path.clone(),
                render_article_page(article, author),
            ));
        }

        // Collection index + chapter pages
        for collection in &author.collections {
            pages.push((
                format!("{}/index.html", collection.out_dir),
                render_collection_page(collection, author),
            ));
            for (i, chapter) in collection.chapters.iter().enumerate() {
                let prev = i.checked_sub(1).map(|p| &collection.chapters[p]);
                let next = collection.chapters.get(i + 1);
                pages.push((
                    chapter.out_path.clone(),
                    render_chapter_page(chapter, prev, next, collection, author),
                ));
            }
        }
    }

    pages
}

/// A listable item (article or collection) with optional metadata.
struct ListEntry {
    title: String,
    href: String,
    date: Option<String>,
    summary: Option<String>,
    /// Author name + link to their author page (homepage cards only)
    author: Option<String>,
    author_href: Option<String>,
}

/// Collect top-level article pages: standalone articles + collections
/// (each collection appears as a single entry pointing to its index page).
fn collect_articles(site: &Site) -> Vec<ListEntry> {
    let mut articles = Vec::new();
    for author in &site.authors {
        for a in &author.articles {
            articles.push(ListEntry {
                title: a.title.clone(),
                href: a.out_path.clone(),
                date: a.date.clone(),
                summary: a.summary.clone(),
                author: Some(author.name.clone()),
                author_href: Some(format!("a/{}/index.html", author.name)),
            });
        }
        for c in &author.collections {
            articles.push(ListEntry {
                title: c.name.clone(),
                href: format!("{}/index.html", c.out_dir),
                date: c.date.clone(),
                summary: c.summary.clone(),
                author: Some(author.name.clone()),
                author_href: Some(format!("a/{}/index.html", author.name)),
            });
        }
    }
    articles
}

fn collect_authors(site: &Site) -> Vec<(String, String)> {
    site.authors
        .iter()
        .map(|a| (a.name.clone(), format!("a/{}/index.html", a.name)))
        .collect()
}

/// Escape a string for embedding in a JS string literal (""-quoted).
fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Escape a string for embedding in HTML text or attribute values.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

// ── page skeleton ──

enum ActiveNav {
    Articles,
    Authors,
    None,
}

struct Crumb {
    label: String,
    href: Option<String>,
}

fn page(
    title: &str,
    depth: usize,
    active: ActiveNav,
    breadcrumbs: &[Crumb],
    body_html: &str,
    description: Option<&str>,
) -> String {
    let desc_meta = description.map_or(String::new(), |d| {
        format!(
            "<meta name=\"description\" content=\"{}\">\n",
            html_escape(d)
        )
    });

    let prefix = if depth == 0 {
        String::new()
    } else {
        vec![".."; depth].join("/")
    };
    let home_href = if depth == 0 {
        "index.html".to_string()
    } else {
        format!("{prefix}/index.html")
    };
    let authors_href = if depth == 0 {
        "authors.html".to_string()
    } else {
        format!("{prefix}/authors.html")
    };

    let topbar = format!(
        r#"<header class="topbar">
<a class="site-name" href="{home_href}">开源文艺</a>
<nav class="topnav">
<a href="{home_href}"{articles_active}>文章</a>
<a href="{authors_href}"{authors_active}>作者</a>
</nav>
</header>"#,
        articles_active = match active {
            ActiveNav::Articles => " class=\"active\"",
            _ => "",
        },
        authors_active = match active {
            ActiveNav::Authors => " class=\"active\"",
            _ => "",
        },
    );

    let mut crumb_html = String::new();
    for (i, crumb) in breadcrumbs.iter().enumerate() {
        if i > 0 {
            crumb_html.push_str(" / ");
        }
        if let Some(ref href) = crumb.href {
            crumb_html.push_str(&format!("<a href=\"{href}\">{}</a>", crumb.label));
        } else {
            crumb_html.push_str(&format!("<span>{}</span>", crumb.label));
        }
    }
    let breadcrumb = if breadcrumbs.is_empty() {
        String::new()
    } else {
        format!(r#"<nav class="breadcrumb" aria-label="面包屑">{crumb_html}</nav>"#)
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
{desc_meta}<title>{title} · 开源文艺</title>
<style>{CSS}</style>
</head>
<body>
{topbar}
{breadcrumb}
<main>
{body_html}
</main>
</body>
</html>"#,
        desc_meta = desc_meta,
    )
}

// ── articles page (homepage) & authors page ──

fn render_articles_page(articles: &[ListEntry]) -> String {
    let items = articles
        .iter()
        .map(|e| {
            format!(
                "{{t:{},h:{},d:{},s:{},au:{},ah:{}}}",
                js_string(&e.title),
                js_string(&e.href),
                e.date
                    .as_deref()
                    .map_or_else(|| "null".to_string(), js_string),
                e.summary
                    .as_deref()
                    .map_or_else(|| "null".to_string(), js_string),
                e.author
                    .as_deref()
                    .map_or_else(|| "null".to_string(), js_string),
                e.author_href
                    .as_deref()
                    .map_or_else(|| "null".to_string(), js_string),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let body = format!(
        r#"<p class="subtitle">共 {} 项</p>
<ul id="card-list" class="article-list"></ul>
<div class="shuffle-bar"><button id="shuffle-btn">换一批</button></div>
<script>const items = [{items}];</script>
<script>
(function () {{
  const list = document.getElementById('card-list');
  function shuffle(a) {{
    for (let i = a.length - 1; i > 0; i--) {{
      const j = Math.floor(Math.random() * (i + 1));
      [a[i], a[j]] = [a[j], a[i]];
    }}
    return a;
  }}
  function render() {{
    list.innerHTML = '';
    if (items.length === 0) {{
      const li = document.createElement('li');
      li.className = 'empty-state';
      li.textContent = '暂无内容';
      list.appendChild(li);
      return;
    }}
    shuffle(items).slice(0, 10).forEach(function (it) {{
      const li = document.createElement('li');
      const card = document.createElement('div');
      card.className = 'card';
      const a = document.createElement('a');
      a.href = it.h;
      a.className = 'stretched';
      const t = document.createElement('span');
      t.className = 'card-title';
      t.textContent = it.t;
      a.appendChild(t);
      card.appendChild(a);
      if (it.d) {{
        const span = document.createElement('span');
        span.className = 'list-date';
        span.textContent = ' ' + it.d;
        card.appendChild(span);
      }}
      if (it.au) {{
        const span = document.createElement('span');
        span.className = 'list-author';
        const la = document.createElement('a');
        la.href = it.ah;
        la.textContent = it.au;
        span.appendChild(la);
        card.appendChild(span);
      }}
      if (it.s) {{
        const p = document.createElement('p');
        p.className = 'list-summary';
        p.textContent = it.s;
        card.appendChild(p);
      }}
      li.appendChild(card);
      list.appendChild(li);
    }});
  }}
  document.getElementById('shuffle-btn').addEventListener('click', render);
  render();
}})();
</script>"#,
        articles.len(),
        items = items,
    );

    page("文章", 0, ActiveNav::Articles, &[], &body, None)
}

fn render_authors_page(authors: &[(String, String)]) -> String {
    let items = authors
        .iter()
        .map(|(t, h)| format!("{{t:{},h:{}}}", js_string(t), js_string(h)))
        .collect::<Vec<_>>()
        .join(",");

    let body = format!(
        r#"<p class="subtitle">共 {} 位</p>
<ul id="card-list" class="article-list"></ul>
<div class="shuffle-bar"><button id="shuffle-btn">换一批</button></div>
<script>const items = [{items}];</script>
<script>
(function () {{
  const list = document.getElementById('card-list');
  function shuffle(a) {{
    for (let i = a.length - 1; i > 0; i--) {{
      const j = Math.floor(Math.random() * (i + 1));
      [a[i], a[j]] = [a[j], a[i]];
    }}
    return a;
  }}
  function render() {{
    list.innerHTML = '';
    if (items.length === 0) {{
      const li = document.createElement('li');
      li.className = 'empty-state';
      li.textContent = '暂无内容';
      list.appendChild(li);
      return;
    }}
    shuffle(items).slice(0, 10).forEach(function (it) {{
      const li = document.createElement('li');
      const a = document.createElement('a');
      a.href = it.h;
      a.className = 'card';
      const t = document.createElement('span');
      t.className = 'card-title';
      t.textContent = it.t;
      a.appendChild(t);
      li.appendChild(a);
      list.appendChild(li);
    }});
  }}
  document.getElementById('shuffle-btn').addEventListener('click', render);
  render();
}})();
</script>"#,
        authors.len(),
        items = items,
    );

    page("作者", 0, ActiveNav::Authors, &[], &body, None)
}

// ── author page ──

fn render_author_page(author: &Author) -> String {
    let mut body = format!("<h1>{}</h1>", author.name);

    if let Some(repo) = &author.repo {
        body.push_str(&format!(
            "\n<p class=\"repo-line\">Git 仓库: <a href=\"{repo}\" target=\"_blank\" rel=\"noopener\">{repo}</a></p>"
        ));
    }

    if let Some(ref intro) = author.intro_html {
        body.push_str(&format!("<div class=\"author-intro\">{intro}</div>"));
    }

    // Mixed list: standalone articles + collections, sorted by name (lexicographic)
    let mut items: Vec<ListEntry> = Vec::new();
    for a in &author.articles {
        let href = if a.out_path.ends_with("index.html") {
            // Dir-based article: link directly to index.html (works under file:// too)
            format!(
                "./{}/index.html",
                a.out_dir.split('/').next_back().unwrap_or(&a.out_dir)
            )
        } else {
            format!(
                "./{}",
                a.out_path.split('/').next_back().unwrap_or(&a.out_path)
            )
        };
        items.push(ListEntry {
            title: a.title.clone(),
            href,
            date: a.date.clone(),
            summary: a.summary.clone(),
            author: None,
            author_href: None,
        });
    }
    for c in &author.collections {
        let dir_name = c.out_dir.split('/').next_back().unwrap_or(&c.name);
        items.push(ListEntry {
            title: c.name.clone(),
            href: format!("./{dir_name}/index.html"),
            date: c.date.clone(),
            summary: c.summary.clone(),
            author: None,
            author_href: None,
        });
    }
    items.sort_by(|a, b| a.title.cmp(&b.title));

    if items.is_empty() {
        body.push_str(r#"<p class="empty-state">暂无内容</p>"#);
    } else {
        body.push_str(r#"<ul class="article-list">"#);
        for item in items {
            body.push_str(&format!(
                "<li><a class=\"card\" href=\"{href}\"><span class=\"card-title\">{title}</span>",
                href = item.href,
                title = item.title,
            ));
            if let Some(date) = &item.date {
                body.push_str(&format!(
                    " <span class=\"list-date\">{}</span>",
                    html_escape(date)
                ));
            }
            if let Some(summary) = &item.summary {
                body.push_str(&format!(
                    "<p class=\"list-summary\">{}</p>",
                    html_escape(summary)
                ));
            }
            body.push_str("</a></li>");
        }
        body.push_str("</ul>");
    }

    page(
        &author.name,
        2,
        ActiveNav::None,
        &[
            Crumb {
                label: "首页".into(),
                href: Some("../../index.html".into()),
            },
            Crumb {
                label: author.name.clone(),
                href: None,
            },
        ],
        &body,
        None,
    )
}

// ── article page (standalone) ──

fn render_article_page(article: &Article, author: &Author) -> String {
    let depth = paths::depth(&article.out_path);
    let author_href = paths::relative_href(depth, &format!("a/{}/index.html", author.name));

    let mut body = format!(
        "<h1>{title}</h1>\n<p class=\"author-line\">作者: <a href=\"{author_href}\">{author}</a></p>",
        title = article.title,
        author = author.name,
    );
    if let Some(date) = &article.date {
        body.push_str(&format!(
            "\n<p class=\"article-meta\">{}</p>",
            html_escape(date)
        ));
    }
    if let Some(summary) = &article.summary {
        body.push_str(&format!(
            "\n<p class=\"summary\">{}</p>",
            html_escape(summary)
        ));
    }
    body.push_str(&format!("\n{}", article.html));

    page(
        &article.title,
        depth,
        ActiveNav::None,
        &[
            Crumb {
                label: "首页".into(),
                href: Some(paths::relative_href(depth, "index.html")),
            },
            Crumb {
                label: author.name.clone(),
                href: Some(author_href),
            },
            Crumb {
                label: article.title.clone(),
                href: None,
            },
        ],
        &body,
        article.summary.as_deref(),
    )
}

// ── collection index page ──

fn render_collection_page(collection: &Collection, author: &Author) -> String {
    let depth = paths::depth(&format!("{}/index.html", collection.out_dir));
    let author_href = paths::relative_href(depth, &format!("a/{}/index.html", author.name));

    let mut body = format!(
        "<h1>{}</h1>\n<p class=\"author-line\">作者: <a href=\"{author_href}\">{author}</a></p>",
        collection.name,
        author = author.name,
    );

    if let Some(date) = &collection.date {
        body.push_str(&format!(
            "\n<p class=\"article-meta\">{}</p>",
            html_escape(date)
        ));
    }
    if let Some(summary) = &collection.summary {
        body.push_str(&format!(
            "\n<p class=\"summary\">{}</p>",
            html_escape(summary)
        ));
    }

    body.push_str("<h2>章节</h2>");
    if collection.chapters.is_empty() {
        body.push_str(r#"<p class="empty-state">暂无章节</p>"#);
    } else {
        body.push_str(r#"<ul class="article-list">"#);
        for ch in &collection.chapters {
            let href = ch.out_path.split('/').next_back().unwrap_or(&ch.file_name);
            body.push_str(&format!(
                "<li><a class=\"card\" href=\"./{href}\"><span class=\"card-title\">{}</span></a></li>",
                ch.title
            ));
        }
        body.push_str("</ul>");
    }

    // Attachments (non-image files)
    if !collection.attachments.is_empty() {
        let depth = paths::depth(&format!("{}/index.html", collection.out_dir));
        let prefix = "../".repeat(depth);
        body.push_str("<div class=\"attachments\"><h3>附件</h3><ul>");
        for asset in &collection.attachments {
            let name = asset
                .out_path
                .split('/')
                .next_back()
                .unwrap_or(&asset.out_path);
            let files_href = format!(
                "{prefix}files/{}/{}",
                author.name,
                asset
                    .out_path
                    .strip_prefix(&format!("files/{}/", author.name))
                    .unwrap_or(name)
            );
            body.push_str(&format!("<li><a href=\"{files_href}\">{name}</a></li>"));
        }
        body.push_str("</ul></div>");
    }

    let depth = paths::depth(&format!("{}/index.html", collection.out_dir));
    let author_href = paths::relative_href(depth, &format!("a/{}/index.html", author.name));

    page(
        &collection.name,
        depth,
        ActiveNav::None,
        &[
            Crumb {
                label: "首页".into(),
                href: Some(paths::relative_href(depth, "index.html")),
            },
            Crumb {
                label: author.name.clone(),
                href: Some(author_href),
            },
            Crumb {
                label: collection.name.clone(),
                href: None,
            },
        ],
        &body,
        collection.summary.as_deref(),
    )
}

// ── chapter page ──

fn render_chapter_page(
    chapter: &Chapter,
    prev: Option<&Chapter>,
    next: Option<&Chapter>,
    collection: &Collection,
    author: &Author,
) -> String {
    let depth = paths::depth(&chapter.out_path);

    let col_href = format!(
        "./{}",
        chapter
            .out_path
            .split('/')
            .next_back()
            .map_or("index.html", |_| "index.html")
    );
    let author_href = paths::relative_href(depth, &format!("a/{}/index.html", author.name));

    let mut body = format!(
        "<h1>{title}</h1>\n<p class=\"author-line\">作者: <a href=\"{author_href}\">{author}</a></p>",
        title = chapter.title,
        author = author.name,
    );
    if let Some(date) = &chapter.date {
        body.push_str(&format!(
            "\n<p class=\"article-meta\">{}</p>",
            html_escape(date)
        ));
    }
    if let Some(summary) = &chapter.summary {
        body.push_str(&format!(
            "\n<p class=\"summary\">{}</p>",
            html_escape(summary)
        ));
    }
    body.push_str(&format!("\n{}", chapter.html));

    // Bottom: only prev/next chapter links (within the collection)
    let prev_html = prev.map_or(r#"<span class="placeholder"></span>"#.to_string(), |p| {
        let name = p.out_path.split('/').next_back().unwrap_or(&p.file_name);
        format!("<a href=\"./{name}\">← 上一章</a>")
    });
    let next_html = next.map_or(r#"<span class="placeholder"></span>"#.to_string(), |n| {
        let name = n.out_path.split('/').next_back().unwrap_or(&n.file_name);
        format!("<a href=\"./{name}\">下一章 →</a>")
    });
    let body = format!("{body}\n<nav class=\"chapter-nav\">{prev_html}{next_html}</nav>",);

    page(
        &chapter.title,
        depth,
        ActiveNav::None,
        &[
            Crumb {
                label: "首页".into(),
                href: Some(paths::relative_href(depth, "index.html")),
            },
            Crumb {
                label: author.name.clone(),
                href: Some(author_href),
            },
            Crumb {
                label: collection.name.clone(),
                href: Some(col_href),
            },
            Crumb {
                label: chapter.title.clone(),
                href: None,
            },
        ],
        &body,
        chapter.summary.as_deref(),
    )
}
