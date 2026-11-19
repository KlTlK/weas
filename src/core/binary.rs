use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn find_singbox_binary() -> Option<PathBuf> {
    // 1. Check current directory
    let current_dir_bin = if cfg!(windows) {
        PathBuf::from("sing-box.exe")
    } else {
        PathBuf::from("sing-box")
    };
    if current_dir_bin.exists() {
        return Some(fs::canonicalize(&current_dir_bin).unwrap_or(current_dir_bin));
    }

    // 2. Check next to current executable
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let next_to_exe = if cfg!(windows) {
                parent.join("sing-box.exe")
            } else {
                parent.join("sing-box")
            };
            if next_to_exe.exists() {
                return Some(next_to_exe);
            }
        }
    }

    // 3. Check weas config directory
    if let Ok(config_dir) = crate::config::AppConfig::config_dir() {
        let in_config = if cfg!(windows) {
            config_dir.join("sing-box.exe")
        } else {
            config_dir.join("sing-box")
        };
        if in_config.exists() {
            return Some(in_config);
        }
    }

    // 4. Check system PATH
    let binary_name = if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    };
    if let Ok(path_var) = env::var("PATH") {
        for p in env::split_paths(&path_var) {
            let candidate = p.join(binary_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

pub async fn ensure_singbox_binary() -> Result<PathBuf> {
    if let Some(bin) = find_singbox_binary() {
        return Ok(bin);
    }

    info!("sing-box binary not found locally. Auto-downloading...");
    let target_dir = crate::config::AppConfig::config_dir()?;
    let downloaded_path = download_singbox(&target_dir).await?;
    Ok(downloaded_path)
}

pub async fn download_singbox(target_dir: &Path) -> Result<PathBuf> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let (os_tag, arch_tag, ext) = match (os, arch) {
        ("windows", "x86_64") => ("windows", "amd64", "zip"),
        ("windows", "aarch64") => ("windows", "arm64", "zip"),
        ("linux", "x86_64") => ("linux", "amd64", "tar.gz"),
        ("linux", "aarch64") => ("linux", "arm64", "tar.gz"),
        ("macos", "x86_64") => ("darwin", "amd64", "tar.gz"),
        ("macos", "aarch64") => ("darwin", "arm64", "tar.gz"),
        _ => return Err(anyhow!("Unsupported OS/architecture: {}/{}", os, arch)),
    };

    let version = "1.10.7";
    let filename = format!("sing-box-{}-{}-{}.{}", version, os_tag, arch_tag, ext);
    let download_url = format!(
        "https://github.com/SagerNet/sing-box/releases/download/v{}/{}",
        version, filename
    );

    info!("Fetching sing-box release from: {}", download_url);

    let client = reqwest::Client::builder()
        .user_agent("weas-installer/0.1.0")
        .build()?;

    let resp = client
        .get(&download_url)
        .send()
        .await
        .context("Failed to request sing-box binary")?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to download sing-box release, status: {}",
            resp.status()
        ));
    }

    let bytes = resp.bytes().await?;
    let bin_name = if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    };
    let dest_bin_path = target_dir.join(bin_name);

    if ext == "zip" {
        extract_zip(&bytes, target_dir, bin_name)?;
    } else {
        fs::write(&dest_bin_path, &bytes)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if dest_bin_path.exists() {
            fs::set_permissions(&dest_bin_path, fs::Permissions::from_mode(0o755))?;
        }
    }

    info!("sing-box ready at: {:?}", dest_bin_path);
    Ok(dest_bin_path)
}

fn extract_zip(archive_data: &[u8], target_dir: &Path, expected_bin: &str) -> Result<()> {
    use std::io::Cursor;
    let reader = Cursor::new(archive_data);
    let mut zip = zip::ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with(expected_bin) {
            let outpath = target_dir.join(expected_bin);
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            return Ok(());
        }
    }

    Err(anyhow!(
        "Could not find {} inside the downloaded zip archive",
        expected_bin
    ))
}
