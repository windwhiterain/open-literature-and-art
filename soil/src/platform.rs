//! Platform abstraction: which Git hosting service soil talks to.
//!
//! GitHub keeps its device-flow login and Bearer-token API. Gitee and
//! GitCode share the same v5-style REST API (`/user/repos`, `/forks`,
//! `/pulls`, `access_token` query parameter) and authorization-code login,
//! so they differ only in base URLs and OAuth registration details.

use serde::Deserialize;
use std::path::Path;

/// Supported hosting platforms.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[default]
    Github,
    Gitee,
    GitCode,
}

impl Platform {
    pub const ALL: [Platform; 3] = [Platform::Github, Platform::Gitee, Platform::GitCode];

    /// Display name used in prompts and messages.
    pub fn name(self) -> &'static str {
        match self {
            Platform::Github => "GitHub",
            Platform::Gitee => "Gitee",
            Platform::GitCode => "GitCode",
        }
    }

    /// REST API base URL.
    pub fn api_base(self) -> &'static str {
        match self {
            Platform::Github => "https://api.github.com",
            Platform::Gitee => "https://gitee.com/api/v5",
            Platform::GitCode => "https://api.gitcode.com/api/v5",
        }
    }

    /// Web base URL used for clone URLs and repo pages.
    pub fn web_base(self) -> &'static str {
        match self {
            Platform::Github => "https://github.com",
            Platform::Gitee => "https://gitee.com",
            Platform::GitCode => "https://gitcode.com",
        }
    }

    /// SSH host in the `git@host:owner/repo` URL form.
    pub fn ssh_host(self) -> &'static str {
        match self {
            Platform::Github => "git@github.com",
            Platform::Gitee => "git@gitee.com",
            Platform::GitCode => "git@gitcode.com",
        }
    }

    /// HTTPS clone URL for a repository.
    pub fn repo_https_url(self, owner: &str, repo: &str) -> String {
        format!("{}/{}/{}.git", self.web_base(), owner, repo)
    }

    /// Web page URL for a repository.
    pub fn repo_html_url(self, owner: &str, repo: &str) -> String {
        format!("{}/{}/{}", self.web_base(), owner, repo)
    }

    /// noreply email domain used for the git commit identity.
    pub fn noreply_domain(self) -> &'static str {
        match self {
            Platform::Github => "github.com",
            Platform::Gitee => "gitee.com",
            Platform::GitCode => "gitcode.com",
        }
    }

    /// OAuth client id. GitHub's is a registered public app; Gitee and
    /// GitCode are the OAuth apps registered for soil (callback
    /// `http://127.0.0.1:8765/callback`).
    pub fn client_id(self) -> &'static str {
        match self {
            Platform::Github => "Ov23li32w7a05VLXLUL6",
            Platform::Gitee => "40b7b9fda3d26a24cd5c03fa4aa13af31c820b7f81ac223ab6d43ec63b221c30",
            Platform::GitCode => "3ac917ae63eb44608cfb6d211fd13f27",
        }
    }

    /// OAuth client secret. Only the authorization-code platforms need one;
    /// GitHub's device flow has no secret.
    pub fn client_secret(self) -> &'static str {
        match self {
            Platform::Github => "",
            Platform::Gitee => "da8122b937be4d5f4a4834ed8557dbd2a2eb5aa53baaaeabb1c426708da2e452",
            Platform::GitCode => "e6fc6913abe2418f8e07925ab190a491",
        }
    }

    /// OAuth authorization-code scope string (space-separated, URL-encoded
    /// when sent). GitHub uses the device flow and never sends this.
    pub fn oauth_scope(self) -> &'static str {
        match self {
            Platform::Github => "",
            Platform::Gitee => "projects user_info pull_requests",
            Platform::GitCode => "all_user all_projects all_pr all_repository",
        }
    }

    /// Upstream repository `(owner, repo)` that users submit PRs against on
    /// this platform. GitHub hosts the canonical repo; the domestic upstreams
    /// are the mirrors (the GitHub owner name `windwhiterain` is not
    /// registered on Gitee/GitCode, the mirrors live under the accounts that
    /// created them).
    pub fn upstream(self) -> (&'static str, &'static str) {
        match self {
            Platform::Github => ("windwhiterain", "open-literature-and-art"),
            Platform::Gitee => ("xiaosijicloud", "open-literature-and-art"),
            Platform::GitCode => ("whiting", "open-literature-and-art"),
        }
    }
}

/// Path of the token file for a platform inside the cache dir.
pub fn token_file_name(platform: Platform) -> String {
    match platform {
        Platform::Github => "token_github.txt".to_string(),
        Platform::Gitee => "token_gitee.txt".to_string(),
        Platform::GitCode => "token_gitcode.txt".to_string(),
    }
}

// ── Repo URL parsing ──

/// Parse a repository reference into `(platform, owner, repo)`. Accepts:
/// - `git@host:owner/repo[.git]`
/// - `https://host/owner/repo[.git]`
/// - bare `owner/repo` (assumes GitHub, the historical default)
pub fn parse_repo_url(url: &str) -> Result<(Platform, String, String), String> {
    let mut s = url.trim().trim_end_matches('/');

    if let Some(rest) = s.strip_prefix("git@") {
        let (host, path) = rest
            .split_once(':')
            .ok_or_else(|| format!("仓库 URL 格式不正确: {url}"))?;
        let platform = platform_for_host(host, url)?;
        let path = path.strip_suffix(".git").unwrap_or(path);
        return split_owner_repo(path, url).map(|(o, r)| (platform, o.to_string(), r.to_string()));
    }

    s = s.strip_suffix(".git").unwrap_or(s);

    if let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        let host_end = rest.find('/').unwrap_or(rest.len());
        let host = &rest[..host_end];
        let path = &rest[host_end..];
        let platform = platform_for_host(host, url)?;
        return split_owner_repo(path, url).map(|(o, r)| (platform, o.to_string(), r.to_string()));
    }

    split_owner_repo(s, url).map(|(o, r)| (Platform::Github, o.to_string(), r.to_string()))
}

fn platform_for_host(host: &str, original: &str) -> Result<Platform, String> {
    let host = host.trim_end_matches('/');
    match host {
        "github.com" => Ok(Platform::Github),
        "gitee.com" => Ok(Platform::Gitee),
        "gitcode.com" => Ok(Platform::GitCode),
        _ => Err(format!("不支持的仓库平台「{host}」，仓库 URL: {original}")),
    }
}

fn split_owner_repo<'a>(path: &'a str, original: &str) -> Result<(&'a str, &'a str), String> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() == 2 {
        Ok((parts[0], parts[1]))
    } else {
        Err(format!("仓库 URL 格式不正确: {original}"))
    }
}

// ── Platform preference persistence ──
//
// `{cache_dir}/config.toml` holds one line: `platform = "gitee"`. Written by
// hand to avoid pulling in a toml dependency for a single key.

/// Load the platform remembered from a previous run, if any.
pub fn load_saved_platform(cache_dir: &Path) -> Option<Platform> {
    let text = std::fs::read_to_string(cache_dir.join("config.toml")).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("platform") {
            let rest = rest.trim_start_matches('=').trim().trim_matches('"');
            if let Some(p) = Platform::from_name(rest) {
                return Some(p);
            }
        }
    }
    None
}

/// Remember the chosen platform for future runs.
pub fn save_platform(cache_dir: &Path, platform: Platform) -> Result<(), String> {
    let text = format!("platform = \"{}\"\n", platform.name().to_ascii_lowercase());
    std::fs::write(cache_dir.join("config.toml"), text)
        .map_err(|e| format!("无法保存平台设置: {e}"))
}

impl Platform {
    /// Parse from the lowercase display name used in config.toml.
    pub fn from_name(s: &str) -> Option<Platform> {
        match s.trim().to_ascii_lowercase().as_str() {
            "github" => Some(Platform::Github),
            "gitee" => Some(Platform::Gitee),
            "gitcode" => Some(Platform::GitCode),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_urls() {
        assert_eq!(
            parse_repo_url("https://github.com/windwhiterain/open-literature-and-art"),
            Ok((
                Platform::Github,
                "windwhiterain".into(),
                "open-literature-and-art".into()
            ))
        );
        assert_eq!(
            parse_repo_url("https://gitee.com/foo/bar.git/"),
            Ok((Platform::Gitee, "foo".into(), "bar".into()))
        );
        assert_eq!(
            parse_repo_url("https://gitcode.com/a/b"),
            Ok((Platform::GitCode, "a".into(), "b".into()))
        );
    }

    #[test]
    fn parse_ssh_urls() {
        assert_eq!(
            parse_repo_url("git@github.com:windwhiterain/repo.git"),
            Ok((Platform::Github, "windwhiterain".into(), "repo".into()))
        );
        assert_eq!(
            parse_repo_url("git@gitee.com:user/文仓库"),
            Ok((Platform::Gitee, "user".into(), "文仓库".into()))
        );
    }

    #[test]
    fn parse_bare_owner_repo_defaults_to_github() {
        assert_eq!(
            parse_repo_url("owner/repo"),
            Ok((Platform::Github, "owner".into(), "repo".into()))
        );
    }

    #[test]
    fn parse_rejects_unknown_host() {
        assert!(parse_repo_url("https://bitbucket.org/a/b").is_err());
        assert!(parse_repo_url("not-a-url").is_err());
    }

    #[test]
    fn url_builders() {
        assert_eq!(
            Platform::Gitee.repo_https_url("u", "r"),
            "https://gitee.com/u/r.git"
        );
        assert_eq!(
            Platform::GitCode.repo_html_url("u", "r"),
            "https://gitcode.com/u/r"
        );
    }

    #[test]
    fn token_file_names() {
        assert_eq!(token_file_name(Platform::Github), "token_github.txt");
        assert_eq!(token_file_name(Platform::Gitee), "token_gitee.txt");
        assert_eq!(token_file_name(Platform::GitCode), "token_gitcode.txt");
    }

    #[test]
    fn platform_name_roundtrip() {
        for p in Platform::ALL {
            assert_eq!(Platform::from_name(p.name()), Some(p));
        }
        assert_eq!(Platform::from_name("GitCode"), Some(Platform::GitCode));
        assert_eq!(Platform::from_name("gitlab"), None);
    }

    #[test]
    fn upstream_per_platform() {
        assert_eq!(
            Platform::Github.upstream(),
            ("windwhiterain", "open-literature-and-art")
        );
        assert_eq!(
            Platform::Gitee.upstream(),
            ("xiaosijicloud", "open-literature-and-art")
        );
        assert_eq!(
            Platform::GitCode.upstream(),
            ("whiting", "open-literature-and-art")
        );
    }
}
