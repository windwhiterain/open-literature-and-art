mod git;
mod github;
mod wizard;

const UPSTREAM_OWNER: &str = "open-literature-and-art";
const UPSTREAM_REPO: &str = "open-literature-and-art";

fn main() {
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
            println!("Pull Request: {}", result.pr_url);
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
