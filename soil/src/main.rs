mod git;
mod github;
mod meta;
mod wizard;

use std::sync::atomic::{AtomicBool, Ordering};

/// Set when the user presses Ctrl+C. The handler swallows the signal so the
/// program keeps running — Ctrl+C is the terminal's copy shortcut, not quit.
/// Exit is via Esc or `q` instead.
pub static CTRL_C_PRESSED: AtomicBool = AtomicBool::new(false);

const UPSTREAM_OWNER: &str = "windwhiterain";
const UPSTREAM_REPO: &str = "open-literature-and-art";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("-h" | "--help" | "help") => print_help(),
        Some("-V" | "--version") => println!("soil {}", env!("CARGO_PKG_VERSION")),
        Some("submit") => run_submit(),
        Some("preview") => wizard::run_preview(args.get(2).map(String::as_str)),
        Some("meta") => meta::run(),
        Some(other) => {
            eprintln!("未知命令: {other}");
            eprintln!();
            print_help();
            std::process::exit(2);
        }
    }
}

/// `soil submit`: the full submission wizard (login → manuscript repo →
/// fork/direct push → submodule → PR → local preview).
fn run_submit() {
    // Capture Ctrl+C: don't let it terminate the process (it's copy, not quit).
    if let Err(e) = ctrlc::set_handler(|| {
        CTRL_C_PRESSED.store(true, Ordering::SeqCst);
    }) {
        eprintln!("警告: 无法注册 Ctrl+C 处理器: {e}");
    }

    // ── Step 0: Environment checks ──

    // 0a. Check git is installed
    if !git::is_git_installed() {
        git::print_git_install_guide();
        std::process::exit(1);
    }

    // 0b. Set up persistent cache directory
    let cache_dir = match dirs::data_dir() {
        Some(d) => d.join("soil"),
        None => {
            eprintln!("无法确定系统数据目录");
            std::process::exit(1);
        }
    };

    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("无法创建缓存目录 {}: {e}", cache_dir.display());
        std::process::exit(1);
    }

    // ── Run the wizard ──
    match wizard::run(&cache_dir, UPSTREAM_OWNER, UPSTREAM_REPO) {
        Ok(result) => {
            println!("作者「{}」的文稿已成功提交！", result.author_name);
            if let Some(url) = result.pr_url {
                println!("Pull Request: {url}");
            } else {
                println!("已直接推送到上游仓库 v1 分支（无需 Pull Request）。");
            }
        }
        Err(e) => {
            if e == "用户取消" {
                println!("已取消。");
                return;
            }
            eprintln!("错误: {e}");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("soil {} — 开源文艺投稿工具", env!("CARGO_PKG_VERSION"));
    println!();
    println!("用法:");
    println!("  soil submit            提交文稿（登录 → 选择文稿仓库 → 提交 → 本地预览）");
    println!(
        "  soil preview [目录]    本地构建网站并在浏览器预览，不做任何 git 操作（默认当前目录）"
    );
    println!("  soil meta              为当前目录的文稿生成 .soil.toml 元数据模板");
    println!("  soil help              显示本帮助");
    println!("  soil -h, --help        显示本帮助");
    println!("  soil -V, --version     显示版本号");
}
