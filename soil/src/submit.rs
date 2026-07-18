use anyhow::Context;
use std::process::Command;

use crate::build;

pub fn run(
    source: Option<&str>,
    author: &str,
    token: &str,
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
    if !status.success() {
        anyhow::bail!("git add failed");
    }

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
    if !status.success() {
        anyhow::bail!("git commit failed");
    }

    // Push to fork
    // Fork management via GitHub API
    let user = github_username(token)?;
    let (owner, repo) = parse_repo_owner();

    ensure_fork(token, &owner, &repo, &user)?;

    let fork_url = format!("https://{}:{}@github.com/{}/{}.git", user, token, user, repo);

    // Add or update fork remote
    let remote_exists = Command::new("git")
        .args(["-C", project_root.to_str().unwrap(), "remote", "get-url", "soil-fork"])
        .output()
        .is_ok();

    if remote_exists {
        Command::new("git")
            .args([
                "-C",
                project_root.to_str().unwrap(),
                "remote",
                "set-url",
                "soil-fork",
                &fork_url,
            ])
            .status()
            .context("Failed to update soil-fork remote")?;
    } else {
        Command::new("git")
            .args([
                "-C",
                project_root.to_str().unwrap(),
                "remote",
                "add",
                "soil-fork",
                &fork_url,
            ])
            .status()
            .context("Failed to add soil-fork remote")?;
    }

    let status = Command::new("git")
        .args([
            "-C",
            project_root.to_str().unwrap(),
            "push",
            "soil-fork",
            "master",
        ])
        .status()
        .context("git push failed")?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    // Create PR
    let pr_resp: serde_json::Value = ureq::post(&format!(
        "https://api.github.com/repos/{}/{}/pulls",
        owner, repo
    ))
    .set("Authorization", &format!("Bearer {}", token))
    .set("User-Agent", "soil-cli")
    .send_json(ureq::json!({
        "title": commit_msg,
        "head": format!("{}:master", user),
        "base": "master",
        "body": format!("投稿 by {}", author),
    }))
    .context("Failed to create PR")?
    .into_json()?;

    let pr_url = pr_resp["html_url"].as_str().unwrap_or("unknown");
    println!("PR created: {}", pr_url);

    Ok(())
}

fn github_username(token: &str) -> anyhow::Result<String> {
    let resp: serde_json::Value = ureq::get("https://api.github.com/user")
        .set("Authorization", &format!("Bearer {}", token))
        .set("User-Agent", "soil-cli")
        .call()
        .context("Failed to get GitHub user. Check your token.")?
        .into_json()?;

    resp["login"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("Invalid token: could not get username"))
}

fn parse_repo_owner() -> (String, String) {
    let url = env!("SOIL_PROJECT_URL");
    let url = url
        .trim_end_matches(".git")
        .trim_end_matches('/');
    let parts: Vec<&str> = url.split('/').collect();
    let owner = parts.get(parts.len().saturating_sub(2)).unwrap_or(&"");
    let repo = parts.last().unwrap_or(&"");
    (owner.to_string(), repo.to_string())
}

fn ensure_fork(
    token: &str,
    owner: &str,
    repo: &str,
    user: &str,
) -> anyhow::Result<()> {
    let check_resp = ureq::get(&format!(
        "https://api.github.com/repos/{}/{}",
        user, repo
    ))
    .set("Authorization", &format!("Bearer {}", token))
    .set("User-Agent", "soil-cli")
    .call();

    match check_resp {
        Ok(resp) if resp.status() == 200 => {
            println!("Fork exists.");
            Ok(())
        }
        _ => {
            println!("Creating fork of {}/{}...", owner, repo);
            ureq::post(&format!(
                "https://api.github.com/repos/{}/{}/forks",
                owner, repo
            ))
            .set("Authorization", &format!("Bearer {}", token))
            .set("User-Agent", "soil-cli")
            .call()
            .context("Failed to create fork")?;
            println!("Fork created.");
            Ok(())
        }
    }
}
