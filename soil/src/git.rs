use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

/// Check whether `git` is available on the system PATH.
pub fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Which GitHub SSH channel a probe found usable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SshChannel {
    /// Plain SSH on port 22 (`git@github.com`) — GitHub's default.
    Port22,
    /// GitHub's alternate endpoint `ssh.github.com:443`, the official
    /// fallback for networks that block SSH on port 22.
    Port443,
}

/// Whether SSH to GitHub is usable for this user: an SSH key exists, and the
/// key authenticates against GitHub (network reachable, key registered).
/// Probed once per run; result is cached.
pub fn ssh_available() -> bool {
    ssh_channel().is_some()
}

/// The SSH channel that works for this user, if any.
/// Tries port 22 first, then GitHub's alternate endpoint on port 443 —
/// port 22 is the one most often blocked on restricted networks.
fn ssh_channel() -> Option<SshChannel> {
    static CHANNEL: OnceLock<Option<SshChannel>> = OnceLock::new();
    *CHANNEL.get_or_init(probe_ssh)
}

fn probe_ssh() -> Option<SshChannel> {
    // 1. An SSH key must exist
    let Some(home) = dirs::home_dir() else {
        return None;
    };
    let ssh_dir = home.join(".ssh");
    let has_key = [
        "id_ed25519",
        "id_rsa",
        "id_ecdsa",
        "id_ed25519_sk",
        "id_ecdsa_sk",
    ]
    .iter()
    .any(|k| ssh_dir.join(k).exists());
    if !has_key {
        return None;
    }

    // 2. Auth probe: fast, non-interactive, bounded timeout.
    // GitHub replies "successfully authenticated" on stderr (exit code 1).
    fn auth_ok(args: &[&str]) -> bool {
        let output = Command::new("ssh")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new")
            .arg("-o")
            .arg("ConnectTimeout=10")
            .arg("-T")
            .args(args)
            .stdout(std::process::Stdio::null())
            .output();
        match output {
            Ok(o) => String::from_utf8_lossy(&o.stderr).contains("successfully authenticated"),
            Err(_) => false,
        }
    }

    if auth_ok(&["git@github.com"]) {
        return Some(SshChannel::Port22);
    }
    if auth_ok(&["-p", "443", "git@ssh.github.com"]) {
        return Some(SshChannel::Port443);
    }
    None
}

/// Make every `https://github.com/...` fetch/push in this repo go over SSH
/// instead (local config only, never committed). Used when SSH is available
/// and HTTPS is flaky; without it the repo just uses HTTPS as-is.
///
/// When only the port-443 channel works, additionally configure
/// `core.sshCommand` so the ssh invoked by git actually reaches
/// `ssh.github.com:443` (which the plain `git@github.com` URL would miss).
pub fn configure_ssh_insteadof(repo_dir: &Path) -> Result<(), String> {
    run_git(
        &[
            "config",
            "url.git@github.com:.insteadOf",
            "https://github.com/",
        ],
        Some(repo_dir),
    )?;
    if ssh_channel() == Some(SshChannel::Port443) {
        run_git(
            &[
                "config",
                "core.sshCommand",
                "ssh -o HostName=ssh.github.com -o Port=443",
            ],
            Some(repo_dir),
        )?;
    }
    Ok(())
}

/// Print installation instructions for git and return false.
pub fn print_git_install_guide() {
    eprintln!("未检测到 Git。请先安装 Git：");
    eprintln!("  macOS:   brew install git  或访问 https://git-scm.com/downloads/mac");
    eprintln!("  Windows: 访问 https://git-scm.com/downloads/win 下载安装包");
    eprintln!("  Linux:   sudo apt install git  (Debian/Ubuntu)");
    eprintln!("           或 sudo yum install git  (RHEL/CentOS)");
    eprintln!();
    eprintln!("Windows 用户安装时需选择「Git from the command line」选项。");
}

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("无法启动 git: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        Err(format!("git {} 失败: {msg}", args.join(" ")))
    }
}

/// Whether a git error looks like a transient network failure worth retrying.
fn is_network_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("unable to access")
        || lower.contains("connection")
        || lower.contains("recv failure")
        || lower.contains("timeout")
        || lower.contains("reset")
        || lower.contains("ssl")
        || lower.contains("tls")
        || lower.contains("could not resolve host")
        || lower.contains("network is unreachable")
        || lower.contains("failed to connect")
        || lower.contains("host key verification failed")
}

/// Run a git command that needs the network, retrying forever on transient
/// network failures (flaky connections reset pushes/fetches). Prints the
/// attempt count so the user sees it is still working. Ctrl+C is swallowed
/// (it is the copy shortcut) — the retry loop just keeps going.
fn run_git_network(args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match run_git(args, cwd) {
            Ok(out) => return Ok(out),
            Err(e) if is_network_error(&e) => {
                println!("网络不稳定（第 {attempt} 次失败），3 秒后自动重试...");
                if crate::CTRL_C_PRESSED.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    println!("[提示] Ctrl+C 是复制快捷键，已忽略，重试继续中。");
                }
                std::thread::sleep(Duration::from_secs(3));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Clone a repository at a specific branch (e.g. `v1`).
/// Shallow and single-branch: the work copy only ever needs the tip of the
/// one branch it operates on, and cloning only that commit instead of the
/// full history makes the initial clone much smaller — important on flaky
/// networks where a full-history clone is the slowest, most failure-prone
/// step. Later fetch/pull deepen the clone automatically as needed.
pub fn clone_branch(url: &str, branch: &str, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    let _ = std::fs::create_dir_all(parent);
    run_git_network(
        &[
            "clone",
            "-b",
            branch,
            "--depth",
            "1",
            "--single-branch",
            url,
            &dest.to_string_lossy(),
        ],
        None,
    )?;
    Ok(())
}

/// Ensure the given branch is checked out, creating it from `origin/<branch>`
/// when it does not exist locally yet.
pub fn checkout_branch(branch: &str, repo_dir: &Path) -> Result<(), String> {
    if run_git(&["checkout", branch], Some(repo_dir)).is_ok() {
        return Ok(());
    }
    run_git(
        &["checkout", "-b", branch, &format!("origin/{branch}")],
        Some(repo_dir),
    )?;
    Ok(())
}

/// Fetch all branches from the `origin` remote.
pub fn fetch_origin(repo_dir: &Path) -> Result<(), String> {
    run_git_network(&["fetch", "origin"], Some(repo_dir))?;
    Ok(())
}

/// Ensure an `upstream` remote exists, pointing at the given URL.
pub fn ensure_upstream_remote(url: &str, repo_dir: &Path) -> Result<(), String> {
    if run_git(&["remote", "get-url", "upstream"], Some(repo_dir)).is_err() {
        run_git(&["remote", "add", "upstream", url], Some(repo_dir))?;
    }
    Ok(())
}

/// Fetch all branches from the `upstream` remote.
pub fn fetch_upstream(repo_dir: &Path) -> Result<(), String> {
    run_git_network(&["fetch", "upstream"], Some(repo_dir))?;
    Ok(())
}

/// Reset the local `branch` to `upstream/<branch>` and force-sync the fork's
/// `origin/<branch>` to the same commit. Used when the fork is stale or was
/// created before the branch existed.
pub fn sync_branch_from_upstream(branch: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(
        &["checkout", "-B", branch, &format!("upstream/{branch}")],
        Some(repo_dir),
    )?;
    run_git_network(&["push", "-f", "-u", "origin", branch], Some(repo_dir))?;
    Ok(())
}

/// Check whether the directory is already a git repository.
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Initialize a new git repository in the directory.
pub fn init(dir: &Path) -> Result<(), String> {
    run_git(&["init"], Some(dir))?;
    Ok(())
}

/// Rename the current branch to `main`.
pub fn branch_main(dir: &Path) -> Result<(), String> {
    run_git(&["branch", "-M", "main"], Some(dir))?;
    Ok(())
}

/// Point the `origin` remote at `url` (adds it if missing, updates if present).
pub fn add_remote(url: &str, dir: &Path) -> Result<(), String> {
    if is_git_repo(dir) && run_git(&["remote", "get-url", "origin"], Some(dir)).is_ok() {
        run_git(&["remote", "set-url", "origin", url], Some(dir))?;
    } else {
        run_git(&["remote", "add", "origin", url], Some(dir))?;
    }
    Ok(())
}

/// Get the URL of the `origin` remote, if configured.
pub fn remote_url(dir: &Path) -> Option<String> {
    run_git(&["remote", "get-url", "origin"], Some(dir)).ok()
}

/// Whether the index has staged changes (after `git add -A`).
pub fn has_staged_changes(dir: &Path) -> bool {
    match Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(dir)
        .output()
    {
        // exit code 0 = no differences, 1 = differences
        Ok(o) => !o.status.success(),
        Err(_) => true,
    }
}

/// Whether the current branch has commits not yet pushed to its upstream.
/// When false, a push would be a no-op — skip it.
pub fn is_ahead_of_upstream(dir: &Path) -> bool {
    match run_git(&["rev-list", "--count", "@{u}..HEAD"], Some(dir)) {
        Ok(out) => out.trim().parse::<u32>().unwrap_or(0) > 0,
        // No upstream tracking yet (never pushed): be conservative, push.
        Err(_) => true,
    }
}

/// Push the current branch to `origin` and set upstream tracking.
pub fn push_upstream(dir: &Path) -> Result<(), String> {
    run_git_network(&["push", "-u", "origin", "HEAD"], Some(dir))?;
    Ok(())
}

/// Pull latest changes in the given repository directory.
pub fn pull(repo_dir: &Path) -> Result<(), String> {
    run_git_network(&["pull"], Some(repo_dir))?;
    Ok(())
}

/// Merge the remote default branch into the local branch keeping local
/// content: remote history is preserved but its file changes are discarded,
/// so the resulting tree equals the user's folder exactly. Enables a plain
/// (non-force) push over an existing repo with unrelated history.
pub fn pull_remote_ours(repo_dir: &Path) -> Result<(), String> {
    for branch in ["main", "master"] {
        let args = [
            "pull",
            "--allow-unrelated-histories",
            "-s",
            "ours",
            "origin",
            branch,
        ];
        if run_git_network(&args, Some(repo_dir)).is_ok() {
            return Ok(());
        }
    }
    Err("无法合并远端历史：远端分支不是 main 或 master".into())
}

/// Add a git submodule. A stale submodule git directory from a previous
/// failed run (`authors/<name>` in `.git/modules/`) makes git refuse the
/// add — clean it up first and retry.
pub fn submodule_add(url: &str, path: &str, repo_dir: &Path) -> Result<(), String> {
    let target = repo_dir.join(path);
    let modules_dir = repo_dir.join(".git/modules").join(path);
    if modules_dir.exists() || target.join(".git").exists() {
        println!("检测到残留的子模块记录，正在清理...");
        let _ = run_git(&["submodule", "deinit", "-f", path], Some(repo_dir));
        let _ = std::fs::remove_dir_all(&modules_dir);
        let _ = std::fs::remove_dir_all(&target);
    }
    run_git_network(&["submodule", "add", url, path], Some(repo_dir))?;
    Ok(())
}

/// Update an existing submodule to the latest commit of its remote branch.
pub fn submodule_update(path: &str, repo_dir: &Path) -> Result<(), String> {
    run_git_network(
        &["submodule", "update", "--init", "--remote", path],
        Some(repo_dir),
    )?;
    Ok(())
}

/// Check out a submodule to the commit recorded in the work copy (no
/// network — the .gitmodules gitlink is the local-of-record commit).
/// Used before building the site for local preview.
pub fn submodule_init(path: &str, repo_dir: &Path) -> Result<(), String> {
    run_git_network(&["submodule", "update", "--init", path], Some(repo_dir))?;
    Ok(())
}

/// Remove stale submodule remnants — submodule gitlinks or checked-out
/// directories whose `.gitmodules` entry no longer exists. `git add -A`
/// would otherwise re-stage them as gitlinks without a URL, and CI's
/// `submodule update` then fails with "No url found for submodule path".
/// Call before staging; a submodule never registered again after being
/// removed upstream must not sneak back into a commit.
pub fn remove_stale_submodules(repo_dir: &Path) -> Result<(), String> {
    let registered = registered_submodule_paths(repo_dir);

    // Gitlinks in the index whose .gitmodules entry is gone
    // (core.quotePath=false keeps author names like Chinese in raw UTF-8)
    let stage = run_git(
        &["-c", "core.quotePath=false", "ls-files", "--stage"],
        Some(repo_dir),
    )?;
    for line in stage.lines() {
        // Format: "<mode> <sha> <stage>\t<path>"; submodules have mode 160000
        let Some((meta, path)) = line.split_once('\t') else {
            continue;
        };
        if !meta.starts_with("160000") || registered.iter().any(|p| p == path) {
            continue;
        }
        run_git(&["rm", "--cached", "--", path], Some(repo_dir))?;
        println!("已移除不再收录的子模块引用: {path}");
    }

    // Checked-out submodule directories under authors/ with no entry —
    // deleting the gitlink alone is not enough, `git add -A` would re-add
    // the directory (it contains a .git file) as a new gitlink.
    let authors_dir = repo_dir.join("authors");
    if let Ok(entries) = std::fs::read_dir(&authors_dir) {
        for entry in entries.flatten() {
            let rel = format!("authors/{}", entry.file_name().to_string_lossy());
            if registered.contains(&rel) {
                continue;
            }
            let dir = entry.path();
            if dir.join(".git").exists() {
                std::fs::remove_dir_all(&dir)
                    .map_err(|e| format!("无法删除残留的子模块目录 {rel}: {e}"))?;
                println!("已移除不再收录的子模块残留目录: {rel}");
            }
        }
    }
    Ok(())
}

/// Paths of submodules registered in `.gitmodules` (e.g. "authors/whiting").
fn registered_submodule_paths(repo_dir: &Path) -> Vec<String> {
    let Ok(out) = run_git(
        &[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\..*\.path$",
        ],
        Some(repo_dir),
    ) else {
        return Vec::new();
    };
    out.lines()
        .filter_map(|line| line.split_once(' ').map(|(_, v)| v.trim().to_string()))
        .collect()
}

/// Add all changes in the working directory.
pub fn add_all(repo_dir: &Path) -> Result<(), String> {
    run_git(&["add", "-A"], Some(repo_dir))?;
    Ok(())
}

/// Commit staged changes.
pub fn commit(message: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(&["commit", "-m", message], Some(repo_dir))?;
    Ok(())
}

/// Push to the default remote.
pub fn push(repo_dir: &Path) -> Result<(), String> {
    run_git_network(&["push"], Some(repo_dir))?;
    Ok(())
}

/// Set git user.name and user.email for commits in the given repo.
pub fn config_user(name: &str, email: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(&["config", "user.name", name], Some(repo_dir))?;
    run_git(&["config", "user.email", email], Some(repo_dir))?;
    Ok(())
}
