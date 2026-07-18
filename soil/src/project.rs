use anyhow::{anyhow, Context};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soil")
}

fn repo_cache_dir() -> PathBuf {
    cache_dir().join("repo")
}

pub fn project_root(github_account: &str) -> anyhow::Result<PathBuf> {
    if cfg!(feature = "dev") {
        let manifest_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .ok_or_else(|| anyhow!("Cannot determine project root from manifest dir"))?;
        let config_toml = root.join("config.toml");
        if !config_toml.exists() {
            anyhow::bail!(
                "Dev project not found at {}. Build without 'dev' feature for author mode.",
                root.display()
            );
        }
        Ok(root.to_path_buf())
    } else {
        let repo = repo_cache_dir();
        if !repo.join("config.toml").exists() {
            // Remove stale directory from a previous failed clone.
            if repo.exists() {
                fs::remove_dir_all(&repo)
                    .with_context(|| format!("Failed to remove stale dir at {}", repo.display()))?;
            }

            let dir = cache_dir();
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create cache dir at {}", dir.display()))?;

            setup_repo(&repo, github_account)?;
        }
        Ok(repo)
    }
}

fn parse_owner_repo() -> (String, String) {
    let url = env!("SOIL_PROJECT_URL");
    let url = url.trim_end_matches(".git").trim_end_matches('/');
    let parts: Vec<&str> = url.rsplitn(3, '/').collect();
    let owner = parts.get(1).unwrap_or(&"").to_string();
    let repo = parts.get(0).unwrap_or(&"").to_string();
    (owner, repo)
}

fn upstream_url() -> String {
    env!("SOIL_PROJECT_URL").to_string()
}

fn fork_url(account: &str) -> String {
    let (_, repo) = parse_owner_repo();
    format!("https://github.com/{}/{}.git", account, repo)
}

fn setup_repo(dest: &std::path::Path, account: &str) -> anyhow::Result<()> {
    let (owner, repo) = parse_owner_repo();

    if account != owner && has_gh() {
        // `gh repo fork` is idempotent — safe to call if fork already exists.
        println!("Ensuring fork of {}/{}...", owner, repo);
        let status = Command::new("gh")
            .args(["repo", "fork", &format!("{}/{}", owner, repo), "--clone=false"])
            .status();
        // Non-zero is OK (fork already exists); only error if the binary is missing.
        let _ = status;
    } else if account != owner {
        println!("GitHub CLI (gh) not found. Cloning upstream directly — you won't be able to submit PRs.");
    }

    let clone_url = if account != owner && has_gh() {
        fork_url(account)
    } else {
        upstream_url()
    };

    let status = Command::new("git")
        .args(["clone", &clone_url, dest.to_str().unwrap()])
        .status()
        .context("Failed to clone repo. Is git installed?")?;
    anyhow::ensure!(status.success(), "git clone failed");

    // Add upstream remote for pulls (only when working on a fork).
    if account != owner && has_gh() {
        Command::new("git")
            .args([
                "-C", dest.to_str().unwrap(),
                "remote", "add", "upstream", &upstream_url(),
            ])
            .status()?;
    }

    Ok(())
}

fn has_gh() -> bool {
    which::which("gh").is_ok()
}

pub fn git_pull(project: &std::path::Path) -> anyhow::Result<()> {
    if cfg!(feature = "dev") {
        return Ok(());
    }

    // Pull from upstream if available, otherwise origin.
    let remote = if has_remote(project, "upstream") {
        "upstream"
    } else {
        "origin"
    };

    let status = Command::new("git")
        .args(["-C", project.to_str().unwrap(), "pull", "--ff-only", remote, "master"])
        .status()
        .context("git pull failed")?;

    if !status.success() {
        anyhow::bail!("git pull returned non-zero");
    }
    Ok(())
}

fn has_remote(project: &std::path::Path, name: &str) -> bool {
    Command::new("git")
        .args(["-C", project.to_str().unwrap(), "remote", "get-url", name])
        .output()
        .is_ok()
}
