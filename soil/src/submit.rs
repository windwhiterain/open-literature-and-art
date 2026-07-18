use anyhow::Context;
use std::process::Command;

use crate::build;

pub fn run(
    source: Option<&str>,
    author: &str,
    github_account: &str,
    message: Option<&str>,
    project_root: &std::path::Path,
) -> anyhow::Result<()> {
    if let Some(source) = source {
        crate::gen::run(
            std::path::Path::new(source),
            author,
            project_root,
        )?;
    }
    build::run(project_root)?;

    let author_dir = format!("content/{}", author);
    let output = Command::new("git")
        .args([
            "-C",
            project_root.to_str().unwrap(),
            "diff",
            "--stat",
            &author_dir,
        ])
        .output()
        .context("git diff failed")?;

    let diff_stat = String::from_utf8_lossy(&output.stdout);
    if diff_stat.trim().is_empty() {
        println!("Nothing to submit.");
        return Ok(());
    }

    println!("Changes to submit:\n{}", diff_stat);

    let confirm: String = dialoguer::Input::new()
        .with_prompt("Submit these changes? [y/N]")
        .default("N".into())
        .interact_text()?;

    if confirm.to_lowercase() != "y" {
        println!("Cancelled.");
        return Ok(());
    }

    let commit_msg_buf;
    let commit_msg = match message {
        Some(m) => m,
        None => {
            commit_msg_buf = format!("投稿: {}", author);
            &commit_msg_buf
        }
    };

    // Stage and commit
    let status = Command::new("git")
        .args(["-C", project_root.to_str().unwrap(), "add", &author_dir])
        .status()
        .context("git add failed")?;
    anyhow::ensure!(status.success(), "git add failed");

    let status = Command::new("git")
        .args([
            "-C",
            project_root.to_str().unwrap(),
            "commit",
            "-m",
            commit_msg,
        ])
        .status()
        .context("git commit failed")?;
    anyhow::ensure!(status.success(), "git commit failed");

    // Ensure fork exists and push.
    let (owner, repo) = parse_repo_owner();
    ensure_fork(project_root, &owner, &repo, github_account)?;
    push_to_fork(project_root)?;

    // Create PR via gh CLI.
    let status = Command::new("gh")
        .args([
            "pr", "create",
            "--title", commit_msg,
            "--body", &format!("投稿 by {}", author),
            "--base", "master",
            "--head", &format!("{}:master", github_account),
        ])
        .current_dir(project_root)
        .status()
        .context("gh pr create failed. Is the GitHub CLI installed and authenticated?")?;
    anyhow::ensure!(status.success(), "gh pr create failed");

    println!("PR created.");
    Ok(())
}

fn ensure_fork(
    project_root: &std::path::Path,
    owner: &str,
    repo: &str,
    account: &str,
) -> anyhow::Result<()> {
    // If the fork remote already exists, we're done.
    if Command::new("git")
        .args(["-C", project_root.to_str().unwrap(), "remote", "get-url", "soil-fork"])
        .output()
        .is_ok()
    {
        // Update the remote in case fork URL changed.
        let fork_url = github_remote_url(account, repo);
        Command::new("git")
            .args(["-C", project_root.to_str().unwrap(), "remote", "set-url", "soil-fork", &fork_url])
            .status()
            .context("Failed to update soil-fork remote")?;
        return Ok(());
    }

    // Fork via gh CLI.
    println!("Forking {}/{}...", owner, repo);
    let status = Command::new("gh")
        .args(["repo", "fork", &format!("{}/{}", owner, repo), "--clone=false"])
        .status()
        .context("gh repo fork failed. Is the GitHub CLI installed and authenticated?")?;
    anyhow::ensure!(status.success(), "gh repo fork failed");

    // Add fork as remote.
    let fork_url = github_remote_url(account, repo);
    Command::new("git")
        .args(["-C", project_root.to_str().unwrap(), "remote", "add", "soil-fork", &fork_url])
        .status()
        .context("Failed to add soil-fork remote")?;

    Ok(())
}

fn push_to_fork(project_root: &std::path::Path) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["-C", project_root.to_str().unwrap(), "push", "soil-fork", "master"])
        .status()
        .context("git push failed. Is your Git credential helper configured?")?;
    anyhow::ensure!(status.success(), "git push failed");
    Ok(())
}

fn github_remote_url(account: &str, repo: &str) -> String {
    format!("https://github.com/{}/{}.git", account, repo)
}

fn parse_repo_owner() -> (String, String) {
    let url = env!("SOIL_PROJECT_URL");
    let url = url.trim_end_matches(".git").trim_end_matches('/');
    let parts: Vec<&str> = url.split('/').collect();
    let owner = parts.get(parts.len().saturating_sub(2)).unwrap_or(&"");
    let repo = parts.last().unwrap_or(&"");
    (owner.to_string(), repo.to_string())
}
