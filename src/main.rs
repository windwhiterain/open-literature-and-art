use std::env;
use std::path::PathBuf;

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
}
