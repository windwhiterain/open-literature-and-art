use anyhow::Context;
use std::path::PathBuf;
use std::process::Command;

/// Run a zola binary with `--version` and extract the version string.
fn zola_version(bin: &std::path::Path) -> Option<String> {
    let output = Command::new(bin).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output is "zola X.Y.Z" — take the second word.
    stdout.split_whitespace().nth(1).map(|s| s.to_string())
}

/// Check if `zola` is available on system PATH and return its path.
fn find_system_zola() -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        "zola.exe"
    } else {
        "zola"
    };
    which::which(name).ok()
}

/// Install Zola via the system's package manager.
fn try_install_via_pkg_manager() -> anyhow::Result<()> {
    if cfg!(target_os = "windows") {
        println!("Installing Zola via winget...");
        let status = Command::new("winget")
            .args(["install", "--accept-source-agreements", "getzola.zola"])
            .status()
            .context("Failed to run winget")?;
        anyhow::ensure!(status.success(), "winget install failed");
        return Ok(());
    }
    if cfg!(target_os = "macos") {
        println!("Installing Zola via Homebrew...");
        let status = Command::new("brew")
            .args(["install", "zola"])
            .status()
            .context("Failed to run brew")?;
        anyhow::ensure!(status.success(), "brew install failed");
        return Ok(());
    }
    // Linux: try snap first (cross-distro), then flatpak.
    if cfg!(target_os = "linux") {
        println!("Installing Zola via snap...");
        if let Ok(s) = Command::new("sudo")
            .args(["snap", "install", "zola"])
            .status()
        {
            if s.success() {
                return Ok(());
            }
        }
        println!("Installing Zola via flatpak...");
        let status = Command::new("flatpak")
            .args(["install", "-y", "flathub", "org.getzola.zola"])
            .status()
            .context("Failed to run flatpak")?;
        anyhow::ensure!(status.success(), "flatpak install failed");
        return Ok(());
    }
    anyhow::bail!("unsupported platform: no package manager available")
}

pub fn install() -> anyhow::Result<()> {
    try_install_via_pkg_manager()?;
    // Verify it's now on PATH.
    anyhow::ensure!(
        find_system_zola().is_some(),
        "Zola installed but not found on PATH — you may need to restart your shell"
    );
    Ok(())
}

pub fn ensure_zola() -> anyhow::Result<PathBuf> {
    // Already on PATH — use it.
    if let Some(path) = find_system_zola() {
        if zola_version(&path).is_some() {
            return Ok(path);
        }
    }

    // Not found — install via package manager.
    install()?;

    find_system_zola()
        .ok_or_else(|| anyhow::anyhow!("Zola installed but not found on PATH"))
}

pub fn run(args: &[&str], cwd: &std::path::Path) -> anyhow::Result<()> {
    let zola_path = ensure_zola()?;
    let status = Command::new(&zola_path)
        .args(args)
        .current_dir(cwd)
        .status()
        .context("Failed to run zola")?;

    if !status.success() {
        anyhow::bail!("zola exited with error");
    }
    Ok(())
}
