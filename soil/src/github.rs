use crate::platform::{self, Platform};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

pub(crate) const USER_AGENT: &str = "soil";

/// Authenticated API client for one hosting platform.
#[derive(Debug)]
pub struct Auth {
    pub platform: Platform,
    pub token: String,
}

// ── Device Flow (GitHub only) ──

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
}

pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    device_code: String,
    interval: u64,
}

/// Agent that returns 4xx/5xx responses as Ok so the error body
/// (the API's JSON error message) can be read instead of discarded.
pub(crate) fn status_ok_agent() -> ureq::Agent {
    ureq::config::Config::builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
}

/// Step 1: Request a device code from GitHub.
pub fn request_device_code() -> Result<DeviceCode, String> {
    let client_id = Platform::Github.client_id();
    let body = format!("client_id={client_id}&scope=public_repo,user:email");

    let resp = status_ok_agent()
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", USER_AGENT)
        .send(&body)
        .map_err(|e| format!("网络请求失败: {e}"))?;

    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))?;

    if status >= 400 {
        return Err(format!("请求设备验证码失败 (HTTP {status}): {text}"));
    }

    let dc: DeviceCodeResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {e}"))?;

    Ok(DeviceCode {
        user_code: dc.user_code,
        verification_uri: dc.verification_uri,
        device_code: dc.device_code,
        interval: dc.interval,
    })
}

/// Step 2: Poll GitHub for the access token until the user authorizes.
pub fn poll_for_token(dc: &DeviceCode) -> Result<String, String> {
    let client_id = Platform::Github.client_id();
    let poll_interval = Duration::from_secs(dc.interval.max(1));
    let deadline = std::time::Instant::now() + Duration::from_secs(300);

    loop {
        std::thread::sleep(poll_interval);

        // Ctrl+C is copy, not quit — swallow it and keep waiting.
        if crate::CTRL_C_PRESSED.swap(false, std::sync::atomic::Ordering::SeqCst) {
            println!(
                "[提示] Ctrl+C 已被忽略（终端中它是复制快捷键）。授权仍在等待中，按 Esc 可退出。"
            );
        }

        let body = format!(
            "client_id={client_id}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code",
            dc.device_code
        );

        let resp = status_ok_agent()
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", USER_AGENT)
            .send(&body);

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                if std::time::Instant::now() >= deadline {
                    return Err(format!("授权超时: {e}"));
                }
                continue;
            }
        };

        let status = resp.status().as_u16();
        let text = match resp.into_body().read_to_string() {
            Ok(t) => t,
            Err(_) => continue,
        };

        if status >= 400 {
            return Err(format!("授权请求失败 (HTTP {status}): {text}"));
        }

        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(_) => continue,
        };

        if let Some(token) = json.get("access_token").and_then(|t| t.as_str()) {
            return Ok(token.to_string());
        }

        if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
            match error {
                "authorization_pending" => {}
                "slow_down" => {
                    std::thread::sleep(Duration::from_secs(5));
                }
                "expired_token" => return Err("验证码已过期，请重新运行 soil".into()),
                "access_denied" => return Err("用户取消了授权".into()),
                _ => return Err(format!("认证失败: {error}")),
            }
        }

        if std::time::Instant::now() >= deadline {
            return Err("授权超时，请重新运行 soil".into());
        }
    }
}

impl Auth {
    pub fn new(platform: Platform, token: String) -> Self {
        Auth { platform, token }
    }
}

// ── Token persistence (per platform) ──

/// Save the access token to the cache dir so future runs can skip the
/// login flow. Unix: restrict to owner-only (0600).
pub fn save_token(cache_dir: &Path, platform: Platform, token: &str) -> Result<(), String> {
    let path = cache_dir.join(platform::token_file_name(platform));
    std::fs::write(&path, token).map_err(|e| format!("无法保存登录凭证: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Load a previously saved access token for a platform, if any.
pub fn load_token(cache_dir: &Path, platform: Platform) -> Option<String> {
    let path = cache_dir.join(platform::token_file_name(platform));
    let token = std::fs::read_to_string(path).ok()?;
    let token = token.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

// ── Response shapes ──

#[derive(Deserialize, Debug)]
pub struct User {
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Repo {
    #[serde(default)]
    pub platform: Platform,
    pub full_name: String,
    #[serde(default)]
    pub clone_url: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
}

impl Repo {
    /// Clone URL, falling back to the platform's standard https URL shape
    /// when the API response omits it (GitCode does).
    pub fn clone_url(&self) -> String {
        self.clone_url.clone().unwrap_or_else(|| {
            let (owner, repo) = split_full_name(&self.full_name);
            self.platform.repo_https_url(owner, repo)
        })
    }

    /// Web page URL, falling back to the standard shape when omitted.
    pub fn html_url(&self) -> String {
        self.html_url.clone().unwrap_or_else(|| {
            let (owner, repo) = split_full_name(&self.full_name);
            self.platform.repo_html_url(owner, repo)
        })
    }
}

fn split_full_name(full_name: &str) -> (&str, &str) {
    match full_name.split_once('/') {
        Some((o, r)) => (o, r),
        None => (full_name, full_name),
    }
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub html_url: Option<String>,
}

impl PullRequest {
    /// Web page URL of the PR.
    pub fn html_url(&self) -> String {
        self.html_url.clone().unwrap_or_default()
    }
}

// ── API request plumbing ──

/// Extract a readable message from an API error body. GitHub often puts the
/// real reason in `errors[].message` while the top-level `message` stays
/// generic; Gitee/GitCode keep everything in `message`.
fn error_message(text: &str) -> String {
    let err: serde_json::Value =
        serde_json::from_str(text).unwrap_or(serde_json::json!({"message": "unknown"}));
    let mut msg = err["message"]
        .as_str()
        .unwrap_or("unknown error")
        .to_string();
    if let Some(errors) = err["errors"].as_array() {
        for e in errors {
            if let Some(m) = e["message"].as_str() {
                msg.push_str("; ");
                msg.push_str(m);
            }
        }
    }
    msg
}

impl Auth {
    /// Append the `access_token` query parameter used by Gitee/GitCode.
    /// GitHub authenticates via the Authorization header instead.
    fn auth_url(&self, url: &str) -> String {
        match self.platform {
            Platform::Github => url.to_string(),
            _ => {
                if url.contains('?') {
                    format!("{url}&access_token={}", self.token)
                } else {
                    format!("{url}?access_token={}", self.token)
                }
            }
        }
    }

    fn get_json(&self, url: &str) -> Result<String, String> {
        let mut req = status_ok_agent()
            .get(&self.auth_url(url))
            .header("User-Agent", USER_AGENT);
        if self.platform == Platform::Github {
            req = req
                .header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json");
        }
        req.call()
            .map_err(|e| format!("请求失败: {e}"))?
            .into_body()
            .read_to_string()
            .map_err(|e| format!("读取响应失败: {e}"))
    }

    fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<(u16, String), String> {
        let body_str = serde_json::to_string(body).unwrap_or_default();
        let mut req = status_ok_agent()
            .post(&self.auth_url(url))
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/json");
        if self.platform == Platform::Github {
            req = req
                .header("Authorization", &format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json");
        }
        let resp = req.send(&body_str).map_err(|e| format!("请求失败: {e}"))?;

        let status = resp.status().as_u16();
        let text = resp
            .into_body()
            .read_to_string()
            .map_err(|e| format!("读取响应失败: {e}"))?;

        Ok((status, text))
    }

    fn parse_repo(&self, text: &str, what: &str) -> Result<Repo, String> {
        let mut repo: Repo =
            serde_json::from_str(text).map_err(|e| format!("解析{what}信息失败: {e}"))?;
        repo.platform = self.platform;
        Ok(repo)
    }

    /// Get the authenticated user's profile.
    pub fn get_user(&self) -> Result<User, String> {
        let text = self.get_json(&format!("{}/user", self.platform.api_base()))?;
        serde_json::from_str(&text).map_err(|e| format!("解析用户信息失败: {e}"))
    }

    /// Whether the authenticated user has push permission on the repo.
    /// True for maintainers of the upstream repo — they skip fork & PR and
    /// push directly. GitHub exposes `permissions.push` on the repo object;
    /// lists members on the repo object itself.
    pub fn has_push_permission(
        &self,
        owner: &str,
        repo: &str,
        username: &str,
    ) -> Result<bool, String> {
        match self.platform {
            Platform::Github => {
                let text = self.get_json(&format!(
                    "{}/repos/{owner}/{repo}",
                    self.platform.api_base()
                ))?;
                let json: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("解析仓库信息失败: {e}"))?;
                Ok(json["permissions"]["push"].as_bool().unwrap_or(false))
            }
            // Gitee repo objects carry no permissions field; the collaborator
            // permission endpoint returns a top-level `permission` string
            // ("admin"/"push"/"pull").
            Platform::Gitee => {
                let text = self.get_json(&format!(
                    "{}/repos/{owner}/{repo}/collaborators/{username}/permission",
                    self.platform.api_base()
                ))?;
                let json: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("解析仓库权限失败: {e}"))?;
                let perm = json["permission"].as_str().unwrap_or("");
                Ok(perm == "push" || perm == "admin")
            }
            // GitCode repo objects list members (usernames) and the owner;
            // being either is enough for the direct-push path soil needs.
            Platform::GitCode => {
                let text = self.get_json(&format!(
                    "{}/repos/{owner}/{repo}",
                    self.platform.api_base()
                ))?;
                let json: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("解析仓库信息失败: {e}"))?;
                let is_owner = json["owner"]["login"].as_str() == Some(username);
                let is_member = json["members"]
                    .as_array()
                    .map(|m| m.iter().any(|u| u.as_str() == Some(username)))
                    .unwrap_or(false);
                Ok(is_owner || is_member)
            }
        }
    }

    /// Create a new repository.
    ///
    /// Must be PUBLIC: the manuscript repo is added as a git submodule of the
    /// upstream site, and its CI (checkout --recurse-submodules) has no
    /// credentials to pull a private submodule.
    pub fn create_repo(&self, name: &str, description: &str) -> Result<Repo, String> {
        let body = serde_json::json!({
            "name": name,
            "description": description,
            "private": false,
            // No auto_init: the user's local folder pushes the initial commit.
            "auto_init": false,
        });

        let (status, text) =
            self.post_json(&format!("{}/user/repos", self.platform.api_base()), &body)?;

        if status >= 400 {
            return Err(format!(
                "创建仓库失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        self.parse_repo(&text, "仓库")
    }

    /// Fork a repository.
    pub fn fork_repo(&self, owner: &str, repo: &str) -> Result<Repo, String> {
        let url = format!("{}/repos/{owner}/{repo}/forks", self.platform.api_base());
        let (status, text) = self.post_json(&url, &serde_json::json!({}))?;

        if status >= 400 {
            return Err(format!(
                "Fork 仓库失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        self.parse_repo(&text, "Fork")
    }

    /// Create a pull request.
    pub fn create_pr(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        head: &str,
        base: &str,
        body: &str,
    ) -> Result<PullRequest, String> {
        let url = format!("{}/repos/{owner}/{repo}/pulls", self.platform.api_base());
        let payload = serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body,
        });

        let (status, text) = self.post_json(&url, &payload)?;

        if status >= 400 {
            return Err(format!(
                "创建 Pull Request 失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        serde_json::from_str(&text).map_err(|e| format!("解析 Pull Request 信息失败: {e}"))
    }
}
