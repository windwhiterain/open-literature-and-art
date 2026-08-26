use crate::git;
use crate::github::{Auth, PullRequest, Repo, User};
use crate::platform::{self, Platform};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// When `true`, updating an existing author's submodule does NOT create a
/// Pull Request — the scheduled auto-update workflow syncs all submodules
/// daily. Set to `false` to restore the old behaviour where every `soil
/// submit` for an existing author opens a PR.
const AUTO_UPDATE_ENABLED: bool = true;

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

fn section(title: &str) {
    println!();
    println!("═══ {title} ═══");
    println!();
}

// ── Step results ──

pub struct WizardResult {
    pub author_name: String,
    /// PR URL; None when the maintainer pushed directly (no PR needed).
    pub pr_url: Option<String>,
}

pub fn run(cache_dir: &Path) -> Result<WizardResult, String> {
    // Step 1: Welcome
    step_welcome()?;

    // Step 1.5: Choose which hosting platform to submit through
    // (remembered in {cache_dir}/config.toml after the first choice).
    let platform = step_choose_platform(cache_dir)?;
    // The upstream is the canonical repo on GitHub or the mirror on domestic
    // platforms — resolved here so callers don't need per-platform constants.
    let (upstream_owner, upstream_repo) = platform.upstream();

    // Step 2: Login (reuse saved token when still valid)
    let auth = step_login_or_load(cache_dir, platform)?;

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
    let manuscript_repo = step_manuscript(&auth, &user)?;

    // Step 5: Prepare the local work copy on the v1 branch
    // Maintainers (push permission on upstream) clone the upstream directly
    // and skip fork & PR; everyone else forks first.
    let is_maintainer = auth.has_push_permission(upstream_owner, upstream_repo, &user.login)?;
    if is_maintainer {
        println!();
        println!("检测到你拥有「开源文艺」仓库的直接写权限，将直接提交到上游（无需 fork / PR）。");
    }
    let work_dir = cache_dir.join("fork");
    step_prepare_work_copy(
        &auth,
        &user,
        upstream_owner,
        upstream_repo,
        is_maintainer,
        &work_dir,
    )?;

    // Step 6: Decide & submit
    // Repo already listed in authors/ → update; otherwise ask for a new
    // author name, and if the submit conflicts (name taken), ask again.
    let (author_name, is_update, made_change) =
        decide_and_submit(&user, &manuscript_repo, &work_dir)?;

    // Step 7: Regular users open a PR; maintainers are already done.
    // No PR when nothing changed (e.g. re-running with no new manuscript).
    // When auto-update is enabled, update PRs are handled by the
    // scheduled workflow — only new-author PRs are created by soil.
    let pr_url = if is_maintainer || !made_change || (is_update && AUTO_UPDATE_ENABLED) {
        if !made_change {
            println!("没有新的变更，无需创建 Pull Request。");
        }
        None
    } else {
        let pr = create_author_pr(
            &auth,
            &user,
            upstream_owner,
            upstream_repo,
            &manuscript_repo,
            &author_name,
            is_update,
        )?;
        Some(pr.html_url())
    };

    // Success
    println!();
    if let Some(url) = &pr_url {
        println!("╔══════════════════════════════════════╗");
        println!("║  已提交 Pull Request！               ║");
        println!("║  {url}\t║");
        println!("║  请等待审核。审核通过后你的文稿将   ║");
        println!("║  出现在「开源文艺」网站上。         ║");
        println!("╚══════════════════════════════════════╝");
    } else if !is_maintainer && is_update && AUTO_UPDATE_ENABLED && made_change {
        println!(
            "你的文稿已提交到你的 {} 仓库（即刻生效）。",
            platform.name()
        );
        println!();
        println!("*** 注意：你的仓库立刻更新了，但网站不会立刻显示。");
        println!("*** 网站每天自动同步一次所有作者的内容，你的更新会在 24 小时内上线。");
        println!("*** 如果你需要立即上线，请联系管理员或站长。");
    } else if made_change {
        println!("已直接推送到「开源文艺」上游仓库 v1 分支，无需 Pull Request。");
    } else {
        println!("文稿已是最新，没有需要提交的变更。");
    }
    println!();

    // Local preview: build the site and open it in the browser.
    section("本地预览");
    let author_submodule = format!("authors/{author_name}");
    println!("正在拉取你的文稿内容...");
    git::submodule_init(&author_submodule, &work_dir)?;

    match sitegen_build_and_open(&work_dir) {
        Ok(()) => {}
        Err(e) => println!("{e}"),
    }

    Ok(WizardResult {
        author_name,
        pr_url,
    })
}

/// `soil preview [目录]`: build the site from the local `authors/` content
/// and open it in the browser. No git operations at all — the directory must
/// already contain the content (e.g. a checkout of the upstream repo).
pub fn run_preview(dir_arg: Option<&str>) {
    let dir = match dir_arg {
        Some(d) => PathBuf::from(d),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("无法确定当前目录: {e}");
                std::process::exit(1);
            }
        },
    };
    if !dir.join("authors").is_dir() {
        eprintln!(
            "错误: 未找到 authors/ 目录。\n请在「开源文艺」仓库根目录运行 soil preview，\
             或指定目录: soil preview <目录>"
        );
        std::process::exit(1);
    }
    match sitegen_build_and_open(&dir) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("错误: {e}");
            std::process::exit(1);
        }
    }
}

/// Build the site in `work_dir` (via the bundled sitegen library) and open
/// `dist/index.html` in the default browser.
fn sitegen_build_and_open(work_dir: &Path) -> Result<(), String> {
    println!("正在构建网站...");
    let config = sitegen::Config {
        authors_dir: work_dir.join("authors"),
        out_dir: work_dir.join("dist"),
        base_url: String::new(),
        check_links: false,
    };
    let summary = sitegen::run(&config);
    for w in &summary.warnings {
        println!("警告: {w}");
    }
    if summary.pages_written == 0 {
        return Err("网站构建未生成任何页面".into());
    }

    let dist = work_dir
        .join("dist")
        .join("index.html")
        .canonicalize()
        .unwrap_or_else(|_| work_dir.join("dist").join("index.html"));
    println!("正在打开浏览器预览...");
    crate::oauth::open_browser(&dist.to_string_lossy());
    println!("已打开：{}", dist.display());
    Ok(())
}

fn step_welcome() -> Result<(), String> {
    println!();
    println!("══════════════════════════════");
    println!("  🌿 soil  ── 开源文艺投稿工具");
    println!("══════════════════════════════");
    println!();
    println!("soil 将帮助你：");
    println!("  1. 选择投稿平台（GitHub / Gitee / GitCode）");
    println!("  2. 登录平台账号");
    println!("  3. 创建或使用已有的文稿仓库");
    println!("  4. 将你的文稿提交到「开源文艺」网站");
    println!();
    println!("退出：任何输入处按 Esc 即可退出。");
    println!("提示：Ctrl+C 是复制快捷键，不会退出程序。");
    println!();
    Ok(())
}

/// Ask which platform to submit through. Remembered in `{cache_dir}/config.toml`;
/// a previously chosen platform is reused without asking again.
fn step_choose_platform(cache_dir: &Path) -> Result<Platform, String> {
    section("选择投稿平台");

    if let Some(saved) = platform::load_saved_platform(cache_dir) {
        println!(
            "使用上次选择的平台: {}（如需更换，请删除 {} 中的 config.toml 后重试）",
            saved.name(),
            cache_dir.display()
        );
        return Ok(saved);
    }

    println!("你的文稿要提交到哪里？");
    for (i, p) in Platform::ALL.iter().enumerate() {
        println!("  {}. {}", i + 1, p.name());
    }
    let choice = ask("选择平台 (1/2/3)");
    let platform = match choice.trim() {
        "1" => Platform::Github,
        "2" => Platform::Gitee,
        "3" => Platform::GitCode,
        _ => {
            return Err(format!("无效的平台选择: {choice}（请输入 1、2 或 3）"));
        }
    };

    if let Err(e) = platform::save_platform(cache_dir, platform) {
        println!("警告: {e}（下次运行需要重新选择平台）");
    }
    Ok(platform)
}

/// Login to the chosen platform, reusing a previously saved token when it
/// is still valid.
fn step_login_or_load(cache_dir: &Path, platform: Platform) -> Result<Auth, String> {
    section(&format!("登录 {}", platform.name()));

    if let Some(token) = crate::github::load_token(cache_dir, platform) {
        let auth = Auth::new(platform, token);
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

    step_login(cache_dir, platform)
}

fn step_login(cache_dir: &Path, platform: Platform) -> Result<Auth, String> {
    if platform.client_id().is_empty() {
        return Err(format!(
            "尚未配置 {} OAuth App 的 client_id。\n\
             请在 soil/src/platform.rs 的 Platform::client_id() 中设置。",
            platform.name()
        ));
    }

    let token = match platform {
        Platform::Github => {
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
            crate::github::poll_for_token(&dc)?
        }
        Platform::Gitee | Platform::GitCode => crate::oauth::login_with_browser(platform)?,
    };
    println!("授权成功！");

    if let Err(e) = crate::github::save_token(cache_dir, platform, &token) {
        println!("警告: {e}（本次登录仍可使用，下次启动需重新登录）");
    }

    Ok(Auth::new(platform, token))
}

/// Parse a repository reference (URL or `owner/repo`) into a Repo, detecting
/// the hosting platform from the URL.
fn repo_from_url(url: &str) -> Result<Repo, String> {
    let (platform, owner, repo_name) = platform::parse_repo_url(url)?;
    Ok(Repo {
        platform,
        full_name: format!("{owner}/{repo_name}"),
        clone_url: None,
        html_url: None,
    })
}

/// Ask for the manuscript folder first, then decide the repo:
/// already a git repo → reuse it (read `origin`); not a git repo → create one.
/// If the chosen repo name already exists, offer to overwrite it via a
/// force push (not delete-and-recreate). The folder itself is the repo's
/// local working copy: it gets initialized, committed and pushed before
/// returning.
fn step_manuscript(auth: &Auth, user: &User) -> Result<Repo, String> {
    section("文稿目录与仓库");

    let dir = std::env::current_dir().map_err(|e| format!("无法获取当前目录: {e}"))?;
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("使用文稿目录: {}", canonical.display());

    let is_git = git::is_git_repo(&dir);
    let existing_origin = if is_git { git::remote_url(&dir) } else { None };

    // Decide repo: existing git repo → reuse; otherwise create new
    let repo = if let Some(url) = &existing_origin {
        println!("检测到该目录已是 git 仓库（远程: {url}）。");
        repo_from_url(url)?
    } else if is_git {
        println!("检测到该目录已是 git 仓库，但未配置远程仓库(origin)。");
        let example = format!(
            "https://{}/user/repo",
            auth.platform.web_base().trim_start_matches("https://")
        );
        let url = ask(&format!("请输入你的仓库 URL (如 {example}):"));
        repo_from_url(&url)?
    } else {
        println!("该目录还不是 git 仓库，将为你创建新仓库。");
        let repo_name = ask_with_default("仓库名称", "literature");
        let desc = ask_with_default("仓库描述", "我的文学作品");
        println!("正在创建仓库（公开仓库——网站构建需要公开访问你的文稿）...");
        match auth.create_repo(&repo_name, &desc) {
            Ok(repo) => {
                println!("仓库已创建: {}", repo.html_url());
                repo
            }
            Err(e) if e.contains("already exists") => {
                println!(
                    "仓库「{repo_name}」已存在，将用你的文稿内容覆盖（保留远端历史，普通 push）。"
                );
                Repo {
                    platform: auth.platform,
                    full_name: format!("{}/{}", user.login, repo_name),
                    clone_url: None,
                    html_url: None,
                }
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
        let repo_url = repo.clone_url();
        git::add_remote(&repo_url, &dir)?;
    }

    // Configure commit identity
    let display = user.name.as_deref().unwrap_or(&user.login);
    let default_email = format!("soil@users.noreply.{}", auth.platform.noreply_domain());
    let email = user.email.as_deref().unwrap_or(&default_email);
    git::config_user(display, email, &dir)?;

    // If SSH to the platform works for this user, route HTTPS traffic over
    // SSH (local config only) — much more reliable on flaky networks.
    if git::ssh_available(auth.platform) {
        println!("检测到 SSH 可用，将使用 SSH 通道加速推送。");
        git::configure_ssh_insteadof(&dir, auth.platform)?;
    }

    // Commit & push — only push when there is something new to push.
    // (A push with an empty delta still touches the network and can hang on
    // flaky connections; skip it entirely when everything is in sync.)
    println!("正在提交并推送文稿...");
    git::remove_stale_submodules(&dir)?;
    git::add_all(&dir)?;
    if git::has_staged_changes(&dir) {
        git::commit("提交文稿", &dir)?;
    } else {
        println!("没有新的变更需要提交。");
    }
    if git::is_ahead_of_upstream(&dir) {
        println!("正在推送...");
        match git::push_upstream(&dir) {
            Ok(()) => {}
            // Non-network failure: the remote has unrelated history
            // (existing repo). Merge its history keeping local content —
            // the result tree equals the user's folder exactly — then push.
            Err(_) => {
                println!("远端仓库已有其他内容，正在合并其历史（内容以你的目录为准）...");
                git::pull_remote_ours(&dir)?;
                git::push_upstream(&dir)?;
            }
        }
        println!("文稿已推送到 {}", repo.html_url());
    } else {
        println!("文稿已与远端同步，无需推送。");
    }

    Ok(repo)
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
        } else if let Some(path) = &current
            && let Some(rest) = line.strip_prefix("url =")
        {
            result.push((path.clone(), rest.trim().to_string()));
            current = None;
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

/// Prepare the local work copy on the v1 branch:
/// maintainers clone the upstream directly; everyone else forks first.
fn step_prepare_work_copy(
    auth: &Auth,
    user: &User,
    upstream_owner: &str,
    upstream_repo: &str,
    is_maintainer: bool,
    work_dir: &Path,
) -> Result<(), String> {
    section("提交到开源文艺");

    if !work_dir.exists() {
        let (owner, label) = if is_maintainer {
            (upstream_owner.to_string(), "上游仓库".to_string())
        } else {
            println!("正在 Fork {upstream_owner}/{upstream_repo}...");
            match auth.fork_repo(upstream_owner, upstream_repo) {
                Ok(fork) => {
                    println!("Fork 完成: {}", fork.html_url());
                    // Wait a moment for the platform to set up the fork
                    println!("等待平台准备仓库...");
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                Err(e) => {
                    // Fork may already exist
                    eprintln!("提示: {e}");
                    println!("尝试使用已有 Fork...");
                }
            }
            (user.login.clone(), "Fork".to_string())
        };

        let clone_url = auth.platform.repo_https_url(&owner, upstream_repo);
        println!("正在克隆{label}（v1 分支）...");
        git::clone_branch(&clone_url, "v1", work_dir)?;

        // The fork is public, so the clone needs no credentials; keep the
        // stored remote URL free of any token (security).
        git::add_remote(&clone_url, work_dir)?;
    } else if is_maintainer {
        println!("本地已有副本，正在更新（v1 分支）...");
        // Make sure origin points at the upstream (an old cached clone may
        // be a fork), then refresh v1 from it.
        let upstream_url = auth.platform.repo_https_url(upstream_owner, upstream_repo);
        git::add_remote(&upstream_url, work_dir)?;
        git::fetch_origin(work_dir)?;
        git::checkout_branch("v1", work_dir)?;
        git::pull(work_dir)?;
    } else {
        // The fork may be stale or predate the v1 branch: sync local v1 and
        // the fork's origin/v1 from upstream.
        println!("本地已有副本，正在与上游同步（v1 分支）...");
        let upstream_url = auth.platform.repo_https_url(upstream_owner, upstream_repo);
        git::ensure_upstream_remote(&upstream_url, work_dir)?;
        git::fetch_upstream(work_dir)?;
        git::sync_branch_from_upstream("v1", work_dir)?;
    }

    // Configure git identity
    let display = user.name.as_deref().unwrap_or(&user.login);
    let default_email = format!("soil@users.noreply.{}", auth.platform.noreply_domain());
    let email = user.email.as_deref().unwrap_or(&default_email);
    git::config_user(display, email, work_dir)?;

    // Route HTTPS traffic over SSH when available (local config only;
    // .gitmodules URLs stay HTTPS so the site CI can clone without creds).
    if git::ssh_available(auth.platform) {
        println!("检测到 SSH 可用，将使用 SSH 通道加速推送。");
        git::configure_ssh_insteadof(work_dir, auth.platform)?;
    }

    Ok(())
}

/// Decide whether this repo is an update (already in authors/) or a new
/// author, then add/update the submodule and push. Returns the author name,
/// whether it was an update, and whether anything was actually committed
/// (an update with no new manuscript commits makes no changes). On name
/// conflicts the user is asked to pick a different author name and the
/// submit is retried.
///
/// Conflict detection happens here, not on the local fork: the work copy may
/// lag behind upstream, so a locally-clean submission can still collide with
/// newer upstream authors at submit time.
fn decide_and_submit(
    user: &User,
    manuscript_repo: &Repo,
    work_dir: &Path,
) -> Result<(String, bool, bool), String> {
    let modules = read_fork_gitmodules(work_dir);
    let repo_url = manuscript_repo.clone_url();

    if let Some((path, _)) = modules
        .iter()
        .find(|(_, url)| normalize_git_url(url) == normalize_git_url(&repo_url))
    {
        let name = path.strip_prefix("authors/").unwrap_or(path).to_string();
        println!("检测到你的文稿仓库已收录为作者「{name}」，将提交更新。");
        let made_change = submit_submodule(manuscript_repo, &name, true, work_dir)?;
        return Ok((name, true, made_change));
    }

    loop {
        let name = ask_with_default("你的作者名称（将显示在网站上）", &user.login);
        match submit_submodule(manuscript_repo, &name, false, work_dir) {
            Ok(made_change) => return Ok((name, false, made_change)),
            Err(e) if is_name_conflict(&e) => {
                println!("提交失败: {e}");
                println!("可能是作者名称「{name}」与现有作者冲突，请换一个名字。");
            }
            Err(e) => return Err(e),
        }
    }
}

/// Add or update the author submodule in the work copy and push.
/// Returns whether a commit was actually made (false when the submodule was
/// already up to date — nothing changed, so no commit/push happens).
fn submit_submodule(
    manuscript_repo: &Repo,
    author_name: &str,
    is_update: bool,
    work_dir: &Path,
) -> Result<bool, String> {
    let author_dir = format!("authors/{author_name}");
    if is_update {
        println!("正在更新子模块到最新...");
        git::submodule_update(&author_dir, work_dir)?;
    } else {
        println!("正在添加子模块...");
        let submodule_url = manuscript_repo.clone_url();
        git::submodule_add(&submodule_url, &author_dir, work_dir)?;
    }

    // Commit and push — only when something actually changed
    git::remove_stale_submodules(work_dir)?;
    git::add_all(work_dir)?;
    if !git::has_staged_changes(work_dir) {
        println!("子模块已是最新，没有新的变更需要提交。");
        return Ok(false);
    }

    let commit_msg = if is_update {
        format!("更新作者 {author_name} 的文稿")
    } else {
        format!("添加作者 {author_name} 的文稿子模块")
    };
    git::commit(&commit_msg, work_dir)?;
    println!("正在推送...");
    git::push(work_dir)?;

    Ok(true)
}

/// Open a PR from the user's fork (v1 branch) to the upstream v1 branch.
fn create_author_pr(
    auth: &Auth,
    user: &User,
    upstream_owner: &str,
    upstream_repo: &str,
    manuscript_repo: &Repo,
    author_name: &str,
    is_update: bool,
) -> Result<PullRequest, String> {
    section("创建 Pull Request");
    let head = format!("{}:v1", user.login);
    let title = if is_update {
        format!("更新作者 {author_name} 的文稿")
    } else {
        format!("添加作者 {author_name}")
    };
    let body = format!(
        "通过 soil 工具提交。\n\n文稿仓库: {}",
        manuscript_repo.html_url()
    );
    println!("正在创建 Pull Request...");
    auth.create_pr(upstream_owner, upstream_repo, &title, &head, "v1", &body)
}

/// Heuristic: a name collision surfaces as git refusing an existing path
/// (e.g. `already exists in the .gitmodules file` / `already exists in the index`).
fn is_name_conflict(err: &str) -> bool {
    err.contains("already exists")
}
