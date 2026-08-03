use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let mut authors_dir = PathBuf::from("authors");
    let mut out_dir = PathBuf::from("dist");
    let mut base_url = String::new();
    let mut check_links = false;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--authors-dir" => {
                i += 1;
                if i < args.len() {
                    authors_dir = PathBuf::from(&args[i]);
                }
            }
            "--out-dir" => {
                i += 1;
                if i < args.len() {
                    out_dir = PathBuf::from(&args[i]);
                }
            }
            "--base-url" => {
                i += 1;
                if i < args.len() {
                    base_url = args[i].clone();
                }
            }
            "--check-links" => {
                check_links = true;
            }
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let config = sitegen::Config {
        authors_dir,
        out_dir,
        base_url,
        check_links,
    };

    let summary = sitegen::run(&config);

    eprintln!(
        "Built: {} pages, {} assets copied",
        summary.pages_written, summary.assets_copied
    );

    for w in &summary.warnings {
        eprintln!("warning: {w}");
    }

    if !summary.warnings.is_empty() {
        std::process::exit(1);
    }

    open_in_browser(&config.out_dir.join("index.html"));
}

/// Open the built index.html in the default browser.
/// Skipped on CI (GitHub Actions sets CI=true) so the deploy pipeline
/// never tries to launch a browser; failures are ignored like in soil.
fn open_in_browser(index_path: &Path) {
    if env::var_os("CI").is_some() {
        return;
    }
    let path = index_path
        .canonicalize()
        .unwrap_or_else(|_| index_path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    eprintln!("Opening browser: {path}");
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", &path])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
}
