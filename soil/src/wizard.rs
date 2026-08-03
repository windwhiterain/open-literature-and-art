use crate::git;
use crate::github::{Auth, GitHubRepo, GitHubUser, PullRequest};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ── Prompt helpers ──
//
// Exit convention: Esc (or `q` / `quit` / `exit`) quits at any input prompt.
// Ctrl+C is swallowed — in the terminal it is the copy shortcut, not quit.

/// Read one line of input. Ctrl+C interrupts the read but is ignored
/// (the terminal's copy shortcut); returns None only on EOF.
fn read_line() -> Option<String> {
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => return None,
            Ok(_) => return Some(input),
            Err(_) => {
                if crate::CTRL_C_PRESSED.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    println!();
                    println!("[提示] Ctrl+C 已被忽略（终端中它是复制快捷键）。按 Esc 退出程序。");
                }
            }
        }
    }
}

/// The quit command: Esc.
fn is_quit(input: &str) -> bool {
    input.trim().starts_with('\u{1b}')
}

/// Exit the program cleanly.
fn quit() -> ! {
    println!("已退出。");
    std::process::exit(0);
}

fn ask(prompt: &str) -> String {
    print!("{prompt} ");
    let _ = io::stdout().flush();
    match read_line() {
        None => quit(),
        Some(input) => {
            if is_quit(&input) {
                quit();
            }
            input.trim().to_string()
        }
    }
}

fn ask_with_default(prompt: &str, default: &str) -> String {
    print!("{prompt} [{default}] ");
    let _ = io::stdout().flush();
    match read_line() {
        None => quit(),
        Some(input) => {
            if is_quit(&input) {
                quit();
            }
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() {
                default.to_string()
            } else {
                trimmed
            }
        }
    }
}

fn confirm(prompt: &str) -> bool {
    loop {
        let answer = ask(&format!("{prompt} (y/n)"));
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            return true;
        }
        if answer.eq_ignore_ascii_case("n") || answer.eq_ignore_ascii_case("no") {
            return false;
        }
        println!("请输入 y 或 n。");
    }
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

    // Step 2: GitHub login (reuse saved token when still valid)
    let auth = step_login_or_load(cache_dir)?;

    // Step 3: Get user info
    section("获取用户信息");
    let user = auth.get_user()?;
    println!(
        "已登录: {} ({})",
        user.login,
        user.email.as_deref().unwrap_or("无邮箱")
    );

    // Step 4: Manuscript folder + repo
    // The folder is asked first; whether it is already a git repo decides
    // between creating a new repo and reusing the existing one.
    let (manuscript_repo, _force_push) = step_manuscript(&auth, &user)?;

    // Step 5: Fork upstream & clone locally — the local fork is the source
    // of truth for deciding between an update PR and a new author PR.
    let fork_dir = cache_dir.join("fork");
    step_fork_clone(&auth, &user, upstream_owner, upstream_repo, &fork_dir)?;

    // Step 6: Decide & submit
    // Repo already listed in authors/ → update PR; otherwise ask for a new
    // author name, and if the submit conflicts (name taken), ask again.
    let fork_modules = read_fork_gitmodules(&fork_dir);
    let repo_url = format!("https://github.com/{}.git", manuscript_repo.full_name);
    let existing_author = fork_modules
        .iter()
        .find(|(_, url)| normalize_git_url(url) == normalize_git_url(&repo_url))
        .map(|(path, _)| path.strip_prefix("authors/").unwrap_or(path).to_string());

    let (author_name, pr_url) = if let Some(name) = existing_author {
        println!("检测到你的文稿仓库已收录为作者「{name}」，将提交更新。");
        let pr = submit_submodule(
            &auth,
            &user,
            upstream_owner,
            upstream_repo,
            &manuscript_repo,
            &name,
            true,
            &fork_dir,
        )?;
        (name, pr.html_url)
    } else {
        let (name, pr_url) = loop {
            let name = ask_with_default("你的作者名称（将显示在网站上）", &user.login);
            match submit_submodule(
                &auth,
                &user,
                upstream_owner,
                upstream_repo,
                &manuscript_repo,
                &name,
                false,
                &fork_dir,
            ) {
                Ok(pr) => break (name, pr.html_url),
                Err(e) if is_name_conflict(&e) => {
                    println!("提交失败: {e}");
                    println!("可能是作者名称「{name}」与现有作者冲突，请换一个名字。");
                }
                Err(e) => return Err(e),
            }
        };
        (name, pr_url)
    };

    // Success
    println!();
    println!("╔══════════════════════════════════════╗");
    println!("║  已提交 Pull Request！               ║");
    println!("║  {}\t║", pr_url);
    println!("║  请等待审核。审核通过后你的文稿将   ║");
    println!("║  出现在「开源文艺」网站上。         ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Preview tip
    println!("💡 提示：你可以在以下目录预览网站：");
    println!("   {}", fork_dir.display());
    println!("   在该目录下运行 cargo run --release -- --out-dir dist");
    println!("   然后用浏览器打开 dist/index.html");

    Ok(WizardResult {
        author_name,
        pr_url,
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
    println!("退出：任何输入处按 Esc 即可退出。");
    println!("提示：Ctrl+C 是复制快捷键，不会退出程序。");
    println!();
    if !confirm("是否继续？") {
        return Err("用户取消".into());
    }
    Ok(())
}

/// Login to GitHub, reusing a previously saved token when it is still valid.
fn step_login_or_load(cache_dir: &Path) -> Result<Auth, String> {
    section("登录 GitHub");

    if let Some(token) = crate::github::load_token(cache_dir) {
        let auth = Auth::new(token);
        match auth.get_user() {
            Ok(user) => {
                println!("已登录: {}（复用已保存的登录凭证）", user.login);
                return Ok(auth);
            }
            Err(_) => {
                println!("已保存的登录凭证已失效，请重新登录。");
            }
        }
    }

    step_login(cache_dir)
}

fn step_login(cache_dir: &Path) -> Result<Auth, String> {
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

    if let Err(e) = crate::github::save_token(cache_dir, &token) {
        println!("警告: {e}（本次登录仍可使用，下次启动需重新登录）");
    }

    Ok(Auth::new(token))
}

/// Parse a GitHub repository reference (URL or `owner/repo`) into a GitHubRepo.
fn repo_from_url(url: &str) -> Result<GitHubRepo, String> {
    let mut trimmed = url.trim().trim_end_matches('/');
    trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    trimmed = trimmed.strip_prefix("git@github.com:").unwrap_or(trimmed);

    let parts: Vec<&str> = trimmed.split('/').collect();
    let (owner, repo_name) = if let Some(pos) = parts.iter().position(|p| *p == "github.com") {
        let after = &parts[pos + 1..];
        if after.len() < 2 {
            return Err(format!("仓库 URL 格式不正确: {url}"));
        }
        (after[0], after[1])
    } else if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        return Err(format!("仓库 URL 格式不正确: {url}"));
    };

    Ok(GitHubRepo {
        full_name: format!("{owner}/{repo_name}"),
        clone_url: format!("https://github.com/{owner}/{repo_name}.git"),
        html_url: format!("https://github.com/{owner}/{repo_name}"),
    })
}

/// Ask for the manuscript folder first, then decide the repo:
/// already a git repo → reuse it (read `origin`); not a git repo → create one.
/// If the chosen repo name already exists, offer to overwrite it via a
/// force push (not delete-and-recreate). The folder itself is the repo's
/// local working copy: it gets initialized, committed and pushed before
/// returning.
fn step_manuscript(auth: &Auth, user: &GitHubUser) -> Result<(GitHubRepo, bool), String> {
    section("文稿目录与仓库");

    println!("你的文稿文件夹将直接作为仓库的工作副本。");
    let input = ask_with_default("请输入你的文稿文件夹路径（直接回车使用当前目录）", ".");
    let dir = std::env::current_dir()
        .map(|cwd| cwd.join(&input))
        .unwrap_or_else(|_| PathBuf::from(&input));

    if !dir.is_dir() {
        return Err(format!("文件夹不存在: {}", dir.display()));
    }
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("使用文稿目录: {}", canonical.display());

    let is_git = git::is_git_repo(&dir);
    let existing_origin = if is_git { git::remote_url(&dir) } else { None };

    // Decide repo: existing git repo → reuse; otherwise create new
    let (repo, force_push) = if let Some(url) = &existing_origin {
        println!("检测到该目录已是 git 仓库（远程: {url}）。");
        (repo_from_url(url)?, false)
    } else if is_git {
        println!("检测到该目录已是 git 仓库，但未配置远程仓库(origin)。");
        let url = ask("请输入你的 GitHub 仓库 URL (如 https://github.com/user/repo):");
        (repo_from_url(&url)?, false)
    } else {
        println!("该目录还不是 git 仓库，将为你创建新仓库。");
        let repo_name = ask_with_default("仓库名称", "literature");
        let desc = ask_with_default("仓库描述", "我的文学作品");
        println!("正在创建仓库（公开仓库——网站构建需要公开访问你的文稿）...");
        match auth.create_repo(&repo_name, &desc) {
            Ok(repo) => {
                println!("仓库已创建: {}", repo.html_url);
                (repo, false)
            }
            Err(e) if e.contains("already exists") => {
                println!("仓库「{repo_name}」已存在。");
                if !confirm("是否用你的文稿覆盖该仓库的内容？(原内容将被替换)") {
                    return Err("用户取消".into());
                }
                println!("将用 force push 覆盖原仓库。");
                (
                    GitHubRepo {
                        full_name: format!("{}/{}", user.login, repo_name),
                        clone_url: format!("https://github.com/{}/{}.git", user.login, repo_name),
                        html_url: format!("https://github.com/{}/{}", user.login, repo_name),
                    },
                    true,
                )
            }
            Err(e) => return Err(e),
        }
    };

    // Prepare the folder as the repo's working copy
    if !is_git {
        println!("正在初始化 git 仓库...");
        git::init(&dir)?;
        git::branch_main(&dir)?;
    }
    if existing_origin.is_none() {
        let repo_url = format!("https://github.com/{}.git", repo.full_name);
        git::add_remote(&repo_url, &dir)?;
    }

    // Configure commit identity
    let display = user.name.as_deref().unwrap_or(&user.login);
    let email = user
        .email
        .as_deref()
        .unwrap_or("soil@users.noreply.github.com");
    git::config_user(display, email, &dir)?;

    // Commit & push
    println!("正在提交并推送文稿...");
    git::add_all(&dir)?;
    if git::has_staged_changes(&dir) {
        git::commit("提交文稿", &dir)?;
    } else {
        println!("没有新的变更需要提交。");
    }
    println!("正在推送...");
    if force_push {
        git::push_upstream_force(&dir)?;
    } else {
        git::push_upstream(&dir)?;
    }
    println!("文稿已推送到 {}", repo.html_url);

    Ok((repo, force_push))
}

/// Read `.gitmodules` from the local fork clone, returning (path, url) pairs.
fn read_fork_gitmodules(fork_dir: &Path) -> Vec<(String, String)> {
    let path = fork_dir.join(".gitmodules");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_gitmodules(&text)
}

/// Parse `.gitmodules` text into (submodule_path, url) pairs.
fn parse_gitmodules(text: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("[submodule \"") {
            if let Some(path) = rest.strip_suffix("\"]") {
                current = Some(path.to_string());
            }
        } else if let Some(path) = &current {
            if let Some(rest) = line.strip_prefix("url =") {
                result.push((path.clone(), rest.trim().to_string()));
                current = None;
            }
        }
    }
    result
}

/// Normalize a git URL for comparison (strip trailing slash and `.git`).
fn normalize_git_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string()
}

/// Fork the upstream if needed and clone (or update) the local fork copy.
fn step_fork_clone(
    auth: &Auth,
    user: &GitHubUser,
    upstream_owner: &str,
    upstream_repo: &str,
    fork_dir: &Path,
) -> Result<(), String> {
    section("提交到开源文艺");

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

        // Strip the token out of the stored remote URL (security)
        let clean_url = format!("https://github.com/{}/{}.git", user.login, upstream_repo);
        git::add_remote(&clean_url, fork_dir)?;
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

    Ok(())
}

/// Add or update the author submodule in the local fork, push, and open a PR.
///
/// Conflict detection happens here, not on the local fork: the user's fork may
/// lag behind upstream, so a locally-clean submission can still collide with
/// newer upstream authors at submit time. On conflict the caller re-asks for
/// the author name.
fn submit_submodule(
    auth: &Auth,
    user: &GitHubUser,
    upstream_owner: &str,
    upstream_repo: &str,
    manuscript_repo: &GitHubRepo,
    author_name: &str,
    is_update: bool,
    fork_dir: &Path,
) -> Result<PullRequest, String> {
    let author_dir = format!("authors/{author_name}");
    if is_update {
        println!("正在更新子模块到最新...");
        git::submodule_update(&author_dir, fork_dir)?;
    } else {
        println!("正在添加子模块...");
        let submodule_url = format!("https://github.com/{}.git", manuscript_repo.full_name);
        git::submodule_add(&submodule_url, &author_dir, fork_dir)?;
    }

    // Commit and push
    let commit_msg = if is_update {
        format!("更新作者 {author_name} 的文稿")
    } else {
        format!("添加作者 {author_name} 的文稿子模块")
    };
    git::add_all(fork_dir)?;
    git::commit(&commit_msg, fork_dir)?;
    println!("正在推送...");
    git::push(fork_dir)?;

    // Create PR
    section("创建 Pull Request");
    let head = format!("{}:v1", user.login);
    let title = if is_update {
        format!("更新作者 {author_name} 的文稿")
    } else {
        format!("添加作者 {author_name}")
    };
    let body = format!(
        "通过 soil 工具提交。\n\n文稿仓库: {}",
        manuscript_repo.html_url
    );
    println!("正在创建 Pull Request...");
    auth.create_pr(upstream_owner, upstream_repo, &title, &head, "v1", &body)
}

/// Heuristic: a name collision surfaces as git refusing an existing path
/// (e.g. `already exists in the .gitmodules file` / `already exists in the index`).
fn is_name_conflict(err: &str) -> bool {
    err.contains("already exists")
}
