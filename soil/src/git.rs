use std::path::Path;
use std::process::Command;

/// Check whether `git` is available on the system PATH.
pub fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
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
        Err(format!("git {} 失败: {}", args.join(" "), stderr.trim()))
    }
}

/// Clone a repository to a destination directory.
pub fn clone(url: &str, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    let _ = std::fs::create_dir_all(parent);
    run_git(&["clone", url, &dest.to_string_lossy()], None)?;
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

/// Push the current branch to `origin` and set upstream tracking.
pub fn push_upstream(dir: &Path) -> Result<(), String> {
    run_git(&["push", "-u", "origin", "HEAD"], Some(dir))?;
    Ok(())
}

/// Force-push the current branch to `origin`, replacing the remote history.
/// Used when overwriting an existing repo with the same name.
pub fn push_upstream_force(dir: &Path) -> Result<(), String> {
    run_git(&["push", "-u", "origin", "HEAD", "--force"], Some(dir))?;
    Ok(())
}

/// Pull latest changes in the given repository directory.
pub fn pull(repo_dir: &Path) -> Result<(), String> {
    run_git(&["pull"], Some(repo_dir))?;
    Ok(())
}

/// Add a git submodule.
pub fn submodule_add(url: &str, path: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(&["submodule", "add", url, path], Some(repo_dir))?;
    Ok(())
}

/// Update an existing submodule to the latest commit of its remote branch.
pub fn submodule_update(path: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(
        &["submodule", "update", "--init", "--remote", path],
        Some(repo_dir),
    )?;
    Ok(())
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
    run_git(&["push"], Some(repo_dir))?;
    Ok(())
}

/// Set git user.name and user.email for commits in the given repo.
pub fn config_user(name: &str, email: &str, repo_dir: &Path) -> Result<(), String> {
    run_git(&["config", "user.name", name], Some(repo_dir))?;
    run_git(&["config", "user.email", email], Some(repo_dir))?;
    Ok(())
}
