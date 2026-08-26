//! Authorization-code login for platforms without GitHub's device flow
//! (Gitee, GitCode). soil starts a local callback server on
//! `127.0.0.1:8765`, opens the browser to the platform's authorize URL,
//! and exchanges the returned code for an access token.

use crate::platform::Platform;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

/// Local callback port. Must match the redirect_uri registered in the
/// platform's OAuth app (`http://127.0.0.1:8765/callback`).
const CALLBACK_PORT: u16 = 8765;

pub const REDIRECT_URI: &str = "http://127.0.0.1:8765/callback";

const TIMEOUT: Duration = Duration::from_secs(300);

/// Run the authorization-code login for a platform and return the access
/// token. Blocks until the user authorizes in the browser or the 5-minute
/// timeout elapses.
pub fn login_with_browser(platform: Platform) -> Result<String, String> {
    let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).map_err(|e| {
        format!("无法在本机启动授权回调端口 {CALLBACK_PORT}: {e}\n请确认该端口未被其他程序占用。")
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("无法设置回调端口: {e}"))?;

    // A random-looking state guards against CSRF on the redirect back.
    let state = format!("{:?}", std::time::SystemTime::now())
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();

    let scope = urlencode(platform.oauth_scope());
    let auth_url = format!(
        "{}/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={scope}&state={state}",
        platform.web_base(),
        platform.client_id(),
        REDIRECT_URI,
    );

    println!("请在浏览器中完成授权：");
    println!();
    println!("    {auth_url}");
    println!();
    println!("即将自动打开浏览器；如果没有打开，请手动复制上面的网址。");
    println!();
    open_browser(&auth_url);
    println!("等待授权中...（最长等待 5 分钟）");
    println!();

    let code = wait_for_callback(&listener, &state)?;
    exchange_token(platform, &code)
}

/// Wait for the browser to hit the callback URL and return the `code`
/// parameter. Polls a non-blocking listener so Ctrl+C (the terminal's copy
/// shortcut) is noticed and reported instead of killing the wait.
fn wait_for_callback(listener: &TcpListener, state: &str) -> Result<String, String> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if crate::CTRL_C_PRESSED.swap(false, std::sync::atomic::Ordering::SeqCst) {
            println!(
                "[提示] Ctrl+C 已被忽略（终端中它是复制快捷键）。授权仍在等待中，按 Esc 可退出。"
            );
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 8192];
                let n = stream
                    .read(&mut buf)
                    .map_err(|e| format!("读取授权回调失败: {e}"))?;
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let params = callback_params(&req);
                let _ = stream.write_all(success_page().as_bytes());
                let _ = stream.flush();
                match (params.get("code"), params.get("state")) {
                    (Some(code), Some(st)) if st == state => return Ok(code.clone()),
                    (Some(_), Some(_)) => return Err("授权状态校验失败，请重新运行 soil。".into()),
                    _ => return Err("授权回调缺少验证码，请重新运行 soil。".into()),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("授权超时，请重新运行 soil。".into());
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => return Err(format!("等待授权回调失败: {e}")),
        }
    }
}

/// Extract the query parameters from a `GET /callback?code=..&state=..` line.
fn callback_params(request: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(first_line) = request.lines().next() else {
        return map;
    };
    let Some(query) = first_line.split_once('?').map(|(_, q)| q) else {
        return map;
    };
    let query = query.split_whitespace().next().unwrap_or(query);
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

/// Exchange the authorization code for an access token.
///
/// Gitee expects every parameter in the form body; GitCode's docs put
/// client_secret in the form body and the rest in the query string.
fn exchange_token(platform: Platform, code: &str) -> Result<String, String> {
    let token_url = format!("{}/oauth/token", platform.web_base());
    let cid = platform.client_id();
    let secret = platform.client_secret();

    let (url, body) = match platform {
        Platform::Gitee => (
            token_url,
            format!(
                "grant_type=authorization_code&code={code}&client_id={cid}&client_secret={secret}&redirect_uri={REDIRECT_URI}"
            ),
        ),
        Platform::GitCode => (
            format!("{token_url}?grant_type=authorization_code&code={code}&client_id={cid}"),
            format!("client_secret={secret}"),
        ),
        Platform::Github => return Err("GitHub 使用设备授权码登录，不会走到此流程。".into()),
    };

    let resp = crate::github::status_ok_agent()
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", crate::github::USER_AGENT)
        .send(&body)
        .map_err(|e| format!("获取授权令牌失败: {e}"))?;

    let status = resp.status().as_u16();
    let text = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("读取授权响应失败: {e}"))?;

    if status >= 400 {
        return Err(format!("获取授权令牌失败 (HTTP {status}): {text}"));
    }

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析授权响应失败: {e}"))?;

    json.get("access_token")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("授权响应中缺少 access_token: {text}"))
}

fn success_page() -> String {
    let body = "<html><body style='font-family:sans-serif;text-align:center;padding-top:80px'>\
                <h2>✅ 授权成功</h2><p>请关闭此窗口，回到终端继续。</p></body></html>";
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// Percent-encode a scope string (`"a b"` → `"a%20b"`).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Open a URL or local path in the default browser.
pub fn open_browser(url_or_path: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", url_or_path])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url_or_path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(url_or_path)
        .spawn();
}
