use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

// ── OAuth App registration (Settings → Developer settings → OAuth Apps) ──
pub const CLIENT_ID: &str = "Ov23li32w7a05VLXLUL6";

const GITHUB_API: &str = "https://api.github.com";
const USER_AGENT: &str = "soil";

#[derive(Debug)]
pub struct Auth {
    pub token: String,
}

// ── Device Flow ──

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
/// (GitHub's JSON error message) can be read instead of discarded.
fn status_ok_agent() -> ureq::Agent {
    ureq::config::Config::builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
}

/// Step 1: Request a device code from GitHub.
pub fn request_device_code() -> Result<DeviceCode, String> {
    let body = format!("client_id={}&scope=public_repo,user:email", CLIENT_ID);

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
            "client_id={}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code",
            CLIENT_ID, dc.device_code
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
    pub fn new(token: String) -> Self {
        Auth { token }
    }
}

// ── Token persistence ──

/// Save the access token to the cache dir so future runs can skip the
/// device flow. Unix: restrict to owner-only (0600).
pub fn save_token(cache_dir: &Path, token: &str) -> Result<(), String> {
    let path = cache_dir.join("token.txt");
    std::fs::write(&path, token).map_err(|e| format!("无法保存登录凭证: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Load a previously saved access token, if any.
pub fn load_token(cache_dir: &Path) -> Option<String> {
    let path = cache_dir.join("token.txt");
    let token = std::fs::read_to_string(path).ok()?;
    let token = token.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

// ── GitHub REST API helpers ──

#[derive(Deserialize, Debug)]
pub struct GitHubUser {
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GitHubRepo {
    pub full_name: String,
    #[allow(dead_code)]
    pub clone_url: String,
    pub html_url: String,
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub html_url: String,
}

fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

/// Extract a readable message from a GitHub API error body.
/// GitHub often puts the real reason in `errors[].message` (e.g.
/// "name already exists on this account") while the top-level `message`
/// stays generic ("Repository creation failed.").
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

fn get_json(token: &str, url: &str) -> Result<String, String> {
    status_ok_agent()
        .get(url)
        .header("Authorization", &auth_header(token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("请求失败: {e}"))?
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))
}

fn post_json(token: &str, url: &str, body: &serde_json::Value) -> Result<(u16, String), String> {
    let body_str = serde_json::to_string(body).unwrap_or_default();
    let resp = status_ok_agent()
        .post(url)
        .header("Authorization", &auth_header(token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .send(&body_str)
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取响应失败: {e}"))?;

    Ok((status, text))
}

impl Auth {
    /// Get the authenticated user's profile.
    pub fn get_user(&self) -> Result<GitHubUser, String> {
        let text = get_json(&self.token, &format!("{GITHUB_API}/user"))?;
        serde_json::from_str(&text).map_err(|e| format!("解析用户信息失败: {e}"))
    }

    /// Whether the authenticated user has push permission on the repo.
    /// True for maintainers of the upstream repo — they skip fork & PR and
    /// push directly.
    pub fn has_push_permission(&self, owner: &str, repo: &str) -> Result<bool, String> {
        let text = get_json(&self.token, &format!("{GITHUB_API}/repos/{owner}/{repo}"))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("解析仓库信息失败: {e}"))?;
        Ok(json["permissions"]["push"].as_bool().unwrap_or(false))
    }

    /// Create a new repository.
    ///
    /// Must be PUBLIC: the manuscript repo is added as a git submodule of the
    /// upstream site, and its CI (checkout --recurse-submodules) has no
    /// credentials to pull a private submodule. Public also matches the
    /// `public_repo` scope granted by device flow.
    pub fn create_repo(&self, name: &str, description: &str) -> Result<GitHubRepo, String> {
        let body = serde_json::json!({
            "name": name,
            "description": description,
            "private": false,
            // No auto_init: the user's local folder pushes the initial commit.
            "auto_init": false,
        });

        let (status, text) = post_json(&self.token, &format!("{GITHUB_API}/user/repos"), &body)?;

        if status >= 400 {
            return Err(format!(
                "创建仓库失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        serde_json::from_str(&text).map_err(|e| format!("解析仓库信息失败: {e}"))
    }

    /// Fork a repository.
    pub fn fork_repo(&self, owner: &str, repo: &str) -> Result<GitHubRepo, String> {
        let url = format!("{GITHUB_API}/repos/{owner}/{repo}/forks");
        let (status, text) = post_json(&self.token, &url, &serde_json::json!({}))?;

        if status >= 400 {
            return Err(format!(
                "Fork 仓库失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        serde_json::from_str(&text).map_err(|e| format!("解析 Fork 信息失败: {e}"))
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
        let url = format!("{GITHUB_API}/repos/{owner}/{repo}/pulls");
        let payload = serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body,
        });

        let (status, text) = post_json(&self.token, &url, &payload)?;

        if status >= 400 {
            return Err(format!(
                "创建 Pull Request 失败 (HTTP {status}): {}",
                error_message(&text)
            ));
        }

        serde_json::from_str(&text).map_err(|e| format!("解析 Pull Request 信息失败: {e}"))
    }
}
