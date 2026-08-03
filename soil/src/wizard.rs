use crate::git;
use crate::github::{Auth, GitHubRepo, GitHubUser};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ── Prompt helpers ──

fn ask(prompt: &str) -> String {
    print!("{prompt} ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input.trim().to_string()
}

fn ask_with_default(prompt: &str, default: &str) -> String {
    print!("{prompt} [{default}] ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed
    }
}

fn confirm(prompt: &str) -> bool {
    let answer = ask(&format!("{prompt} (y/n)"));
    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
}

fn section(title: &str) {
    println!();
    println!("═══ {title} ═══");
    println!();
}

// ── Step results ──

pub struct WizardResult {
    pub author_name: String,
    pub pr_url: String,
}

pub fn run(
    cache_dir: &Path,
    upstream_owner: &str,
    upstream_repo: &str,
) -> Result<WizardResult, String> {
    // Step 1: Welcome
    step_welcome()?;

    // Step 2: GitHub login
    let auth = step_login()?;

    // Step 3: Get user info
    section("获取用户信息");
    let user = auth.get_user()?;
    println!(
        "已登录: {} ({})",
        user.login,
        user.email.as_deref().unwrap_or("无邮箱")
    );

    // Step 4: Manuscript repo
    let (manuscript_repo, author_name) = step_manuscript_repo(&auth, &user, cache_dir)?;

    // Step 5: Add manuscript files
    let ms_dir = cache_dir.join("manuscripts").join(
        &manuscript_repo
            .full_name
            .split('/')
            .nth(1)
            .unwrap_or(&manuscript_repo.full_name),
    );
    step_add_files(&ms_dir)?;

    // Step 6: Push manuscript changes
    section("推送文稿");
    println!("正在提交并推送文稿...");
    let author_dir = format!("authors/{author_name}");
    git::add_all(&ms_dir)?;
    git::commit(&format!("添加 {author_name} 的文稿"), &ms_dir)?;
    git::push(&ms_dir)?;
    println!("文稿已推送到 {}", manuscript_repo.html_url);

    // Step 7: Fork upstream & add submodule
    let fork_dir = cache_dir.join("fork");
    step_fork_and_submodule(
        &auth,
        &user,
        upstream_owner,
        upstream_repo,
        &manuscript_repo,
        &author_name,
        &author_dir,
        &fork_dir,
    )?;

    // Step 8: Create PR
    section("创建 Pull Request");
    let head = format!("{}:v1", user.login);
    let title = format!("添加作者 {author_name}");
    let body = format!(
        "通过 soil 工具提交。\n\n文稿仓库: {}",
        manuscript_repo.html_url
    );
    println!("正在创建 Pull Request...");
    let pr = auth.create_pr(upstream_owner, upstream_repo, &title, &head, "v1", &body)?;
    println!();
    println!("╔══════════════════════════════════════╗");
    println!("║  已提交 Pull Request！               ║");
    println!("║  {}\t║", pr.html_url);
    println!("║  请等待审核。审核通过后你的文稿将   ║");
    println!("║  出现在「开源文艺」网站上。         ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Step 9: Preview tip
    println!("💡 提示：你可以在以下目录预览网站：");
    println!("   {}", fork_dir.display());
    println!("   在该目录下运行 cargo run --release -- --out-dir dist");
    println!("   然后用浏览器打开 dist/index.html");

    Ok(WizardResult {
        author_name,
        pr_url: pr.html_url,
    })
}

fn step_welcome() -> Result<(), String> {
    println!();
    println!("══════════════════════════════");
    println!("  🌿 soil  ── 开源文艺投稿工具");
    println!("══════════════════════════════");
    println!();
    println!("soil 将帮助你：");
    println!("  1. 登录 GitHub 账号");
    println!("  2. 创建或使用已有的文稿仓库");
    println!("  3. 将你的文稿提交到「开源文艺」网站");
    println!();
    if !confirm("是否继续？") {
        return Err("用户取消".into());
    }
    Ok(())
}

fn step_login() -> Result<Auth, String> {
    section("登录 GitHub");

    if crate::github::CLIENT_ID.is_empty() {
        return Err("尚未配置 GitHub OAuth App 的 client_id。\n\
             请在 soil/src/github.rs 中设置 CLIENT_ID 常量。"
            .into());
    }

    println!("正在请求设备验证码...");
    let dc = crate::github::request_device_code()?;
    println!();
    println!("请在浏览器中打开以下网址：");
    println!();
    println!("    {}", dc.verification_uri);
    println!();
    println!("输入验证码：{}", dc.user_code);
    println!();
    println!("等待授权中...（最长等待 5 分钟）");
    println!();

    let token = crate::github::poll_for_token(&dc)?;
    println!("授权成功！");
    Ok(Auth::new(token))
}

fn step_manuscript_repo(
    auth: &Auth,
    user: &GitHubUser,
    cache_dir: &Path,
) -> Result<(GitHubRepo, String), String> {
    section("文稿仓库");

    println!("你需要一个 GitHub 仓库来存放你的文稿。");
    println!();
    println!("  1. 创建新仓库");
    println!("  2. 使用已有仓库");
    println!();

    let choice = ask_with_default("请选择 (1/2)", "1");

    let repo = match choice.as_str() {
        "2" => {
            let url = ask("请输入你的仓库 URL (如 https://github.com/user/repo):");
            let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
            if parts.len() < 2 {
                return Err("仓库 URL 格式不正确".into());
            }
            let repo_name = parts[parts.len() - 1];
            let owner = parts[parts.len() - 2];

            let clone_url = format!("https://github.com/{owner}/{repo_name}.git");
            let html_url = format!("https://github.com/{owner}/{repo_name}");

            let dest = cache_dir.join("manuscripts").join(repo_name);
            if dest.exists() {
                println!("本地已有克隆，正在更新...");
                git::pull(&dest)?;
            } else {
                println!("正在克隆仓库...");
                let auth_url =
                    format!("https://{}@github.com/{owner}/{repo_name}.git", auth.token,);
                git::clone(&auth_url, &dest)?;
            }

            GitHubRepo {
                full_name: format!("{owner}/{repo_name}"),
                clone_url,
                html_url,
            }
        }
        _ => {
            let repo_name = ask_with_default("仓库名称", "literature");
            let desc = ask_with_default("仓库描述", "我的文学作品");

            println!("正在创建仓库...");
            let repo = auth.create_repo(&repo_name, &desc)?;
            println!("仓库已创建: {}", repo.html_url);

            let dest = cache_dir.join("manuscripts").join(&repo_name);
            if !dest.exists() {
                let auth_url = format!(
                    "https://{}@github.com/{}/{}.git",
                    auth.token, user.login, repo_name
                );
                git::clone(&auth_url, &dest)?;
            }

            // Create a README.md if the repo is empty (aside from auto_init)
            let readme = dest.join("README.md");
            if !readme.exists() {
                std::fs::write(&readme, format!("# {repo_name}\n\n{desc}\n")).ok();
                git::add_all(&dest)?;
                git::commit("初始化文稿仓库", &dest)?;
                git::push(&dest)?;
            }

            repo
        }
    };

    let author_name = ask_with_default("你的作者名称（将显示在网站上）", &user.login);

    // Configure git identity for commits
    let display = user.name.as_deref().unwrap_or(&user.login);
    let email = user
        .email
        .as_deref()
        .unwrap_or("soil@users.noreply.github.com");
    let ms_dir = cache_dir
        .join("manuscripts")
        .join(repo.full_name.split('/').nth(1).unwrap_or(&repo.full_name));
    git::config_user(display, email, &ms_dir)?;

    Ok((repo, author_name))
}

fn step_add_files(ms_dir: &Path) -> Result<(), String> {
    section("添加文稿");

    let author_name;
    {
        // Infer author name from the first existing authors/* dir, or ask
        let authors_dir = ms_dir.join("authors");
        if !authors_dir.exists() {
            std::fs::create_dir_all(&authors_dir).ok();
        }

        let source = ask("请输入你的文稿文件夹路径（包含 .md 文件）:");
        let source_path = PathBuf::from(&source);
        if !source_path.exists() || !source_path.is_dir() {
            return Err(format!("文件夹不存在: {source}"));
        }

        // Get author directory name from the source folder name
        let dir_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());

        let dest_dir = authors_dir.join(&dir_name);
        println!("正在复制文件...");
        git::copy_dir(&source_path, &dest_dir)?;
        println!("已添加: {dir_name}");

        author_name = dir_name;
    }

    // Write .gitmodules for submodule tracking later
    let gitmodules = ms_dir.join(".gitmodules");
    if !gitmodules.exists() {
        std::fs::write(&gitmodules, "").ok();
    }

    git::add_all(ms_dir)?;

    // Check if there's something to commit
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(ms_dir)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if status.trim().is_empty() {
        println!("没有新的变更需要提交。");
    } else {
        let msg = format!("添加 {author_name} 的文稿");
        git::commit(&msg, ms_dir)?;
    }

    loop {
        let more = ask("是否还有其他文稿文件夹需要添加？(y/n)");
        if !more.eq_ignore_ascii_case("y") && !more.eq_ignore_ascii_case("yes") {
            break;
        }

        let source = ask("请输入文稿文件夹路径:");
        let source_path = PathBuf::from(&source);
        if !source_path.exists() || !source_path.is_dir() {
            eprintln!("文件夹不存在，跳过");
            continue;
        }

        let dir_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());

        let dest_dir = ms_dir.join("authors").join(&dir_name);
        println!("正在复制文件...");
        git::copy_dir(&source_path, &dest_dir)?;
        println!("已添加: {dir_name}");

        git::add_all(ms_dir)?;
        git::commit(&format!("添加 {dir_name} 的文稿"), ms_dir)?;
    }

    Ok(())
}

fn step_fork_and_submodule(
    auth: &Auth,
    user: &GitHubUser,
    upstream_owner: &str,
    upstream_repo: &str,
    manuscript_repo: &GitHubRepo,
    author_name: &str,
    author_dir: &str,
    fork_dir: &Path,
) -> Result<(), String> {
    section("提交到开源文艺");

    // Fork the upstream
    println!("正在 Fork {upstream_owner}/{upstream_repo}...");

    if !fork_dir.exists() {
        match auth.fork_repo(upstream_owner, upstream_repo) {
            Ok(fork) => {
                println!("Fork 完成: {}", fork.html_url);
                // Wait a moment for GitHub to set up the fork
                println!("等待 GitHub 准备仓库...");
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
            Err(e) => {
                // Fork may already exist
                eprintln!("提示: {e}");
                println!("尝试使用已有 Fork...");
            }
        }

        let auth_url = format!(
            "https://{token}@github.com/{login}/{repo}.git",
            token = auth.token,
            login = user.login,
            repo = upstream_repo
        );
        println!("正在克隆 Fork...");
        git::clone(&auth_url, fork_dir)?;
    } else {
        println!("本地已有 Fork 副本，正在更新...");
        git::pull(fork_dir)?;
    }

    // Configure git identity in fork
    let display = user.name.as_deref().unwrap_or(&user.login);
    let email = user
        .email
        .as_deref()
        .unwrap_or("soil@users.noreply.github.com");
    git::config_user(display, email, fork_dir)?;

    // Add submodule
    println!("正在添加子模块...");
    let submodule_url = format!("https://github.com/{}.git", manuscript_repo.full_name);
    git::submodule_add(&submodule_url, author_dir, fork_dir)?;

    // Commit and push
    let commit_msg = format!("添加作者 {author_name} 的文稿子模块");
    git::add_all(fork_dir)?;
    git::commit(&commit_msg, fork_dir)?;
    println!("正在推送...");
    git::push(fork_dir)?;

    Ok(())
}
