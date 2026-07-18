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

pub fn project_root() -> anyhow::Result<PathBuf> {
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
            let url = env!("SOIL_PROJECT_URL");
            let dir = cache_dir();
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create cache dir at {}", dir.display()))?;

            let status = Command::new("git")
                .args(["clone", url, repo.to_str().unwrap()])
                .status()
                .context("Failed to clone project repo. Is git installed?")?;

            if !status.success() {
                anyhow::bail!("git clone failed");
            }
        }
        Ok(repo)
    }
}

pub fn git_pull(project: &std::path::Path) -> anyhow::Result<()> {
    if cfg!(feature = "dev") {
        return Ok(());
    }

    let status = Command::new("git")
        .args(["-C", project.to_str().unwrap(), "pull", "--ff-only"])
        .status()
        .context("git pull failed")?;

    if !status.success() {
        anyhow::bail!("git pull returned non-zero");
    }
    Ok(())
}
