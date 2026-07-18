use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;
use tar::Archive;

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soil")
}

fn zola_bin() -> PathBuf {
    let mut path = cache_dir().join("zola");
    if cfg!(target_os = "windows") {
        path.set_extension("exe");
    }
    path
}

#[derive(Deserialize)]
struct ReleaseAsset {
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

fn platform_asset_pattern() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "pc-windows-msvc"
    } else {
        panic!("unsupported platform")
    }
}

pub fn install() -> anyhow::Result<()> {
    let zola_path = zola_bin();

    let response: Release = ureq::get(
        "https://api.github.com/repos/getzola/zola/releases/latest",
    )
    .set("User-Agent", "soil-cli")
    .call()
    .context("Failed to query Zola releases")?
    .into_json()?;

    let pattern = platform_asset_pattern();
    let url = response
        .assets
        .iter()
        .find_map(|a| {
            if a.browser_download_url.contains(pattern) {
                Some(&a.browser_download_url)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow!(
                "No Zola release found for platform '{}' (version {})",
                pattern,
                response.tag_name
            )
        })?;

    let parent = zola_path.parent().unwrap();
    fs::create_dir_all(parent)?;

    let resp = ureq::get(url)
        .set("User-Agent", "soil-cli")
        .call()
        .context("Failed to download Zola")?;

    let mut data = Vec::new();
    resp.into_reader().read_to_end(&mut data)?;

    if url.ends_with(".tar.gz") {
        let tar = GzDecoder::new(io::Cursor::new(&data));
        let mut archive = Archive::new(tar);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.ends_with("zola") || path.ends_with("zola.exe") {
                let out_path = zola_bin();
                entry.unpack(&out_path)?;
                break;
            }
        }
    } else if url.ends_with(".zip") {
        let reader = io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(reader)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name();
            if name.ends_with("zola.exe") || name.ends_with("zola") {
                let out_path = zola_bin();
                let mut out = fs::File::create(&out_path)?;
                io::copy(&mut file, &mut out)?;
                break;
            }
        }
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&zola_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&zola_path, perms)?;
    }

    // Write version
    fs::write(cache_dir().join("zola-version"), &response.tag_name)?;

    println!("Zola {} installed.", response.tag_name);
    Ok(())
}

pub fn ensure_zola() -> anyhow::Result<PathBuf> {
    let zola_path = zola_bin();
    if !zola_path.exists() {
        install()?;
    }
    Ok(zola_path)
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
