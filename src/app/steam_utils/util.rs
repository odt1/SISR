use new_vdf_parser::open_shortcuts_vdf;
use std::{path::PathBuf, process::Command, sync::OnceLock};
use tracing::debug;
use tracing::trace;
use tracing::warn;

use crate::app::steam_utils::cef_debug;

static STEAM_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static LAUNCHED_VIA_STEAM: OnceLock<bool> = OnceLock::new();

pub fn init() {
    let launched_via_steam = std::env::var("SteamGameId").is_ok();
    LAUNCHED_VIA_STEAM.set(launched_via_steam).ok();
    debug!("Launched via Steam: {}", launched_via_steam);
}

pub fn launched_via_steam() -> bool {
    *LAUNCHED_VIA_STEAM.get().unwrap_or(&false)
}

pub fn open_steam_url(url: &str) -> Result<(), std::io::Error> {
    debug!("Opening Steam URL: {}", url);

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/c", "start", "", url]).spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }

    Ok(())
}

pub fn steam_path() -> Option<PathBuf> {
    if let Some(cfg_path) = crate::config::CONFIG
        .get()
        .and_then(|cfg| cfg.steam.steam_path.clone())
    {
        trace!("Using configured Steam path: {}", cfg_path.display());
        return Some(cfg_path);
    }

    // Let's just assume steam path install doesn't change during runtime...
    if let Some(cached_path) = STEAM_PATH.get() {
        return cached_path.clone();
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(steam_key) = hklm.open_subkey("Software\\Valve\\Steam") {
            let Ok(install_path) = steam_key.get_value("SteamPath") as Result<String, _> else {
                return None;
            };
            let path = Some(PathBuf::from(install_path));
            trace!(
                "Found Steam install path {}",
                path.as_ref().unwrap().display()
            );
            STEAM_PATH.set(path.clone()).ok();
            return path;
        }
        None
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home_dir) = directories::BaseDirs::new().map(|bd| bd.home_dir().to_path_buf()) {
            let steam_path = home_dir.join(".steam/steam");
            if steam_path.exists() {
                let path = Some(steam_path);
                trace!(
                    "Found Steam install path {}",
                    path.as_ref().unwrap().display()
                );
                STEAM_PATH.set(path.clone()).ok();
                return path;
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home_dir) = directories::BaseDirs::new().map(|bd| bd.home_dir().to_path_buf()) {
            let steam_path = home_dir.join("Library/Application Support/Steam");
            if steam_path.exists() {
                let path = Some(steam_path);
                trace!(
                    "Found Steam install path {}",
                    path.as_ref().unwrap().display()
                );
                STEAM_PATH.set(path.clone()).ok();
                return path;
            }
        }
        None
    }
}

pub fn active_user_id() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(steam_key) = hklm.open_subkey("Software\\Valve\\Steam\\ActiveProcess") {
            let Ok(user_id) = steam_key.get_value("ActiveUser") as Result<u32, _> else {
                return None;
            };
            trace!("Found active Steam user ID: {}", user_id);
            return Some(user_id);
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Untested AI code, but wel'll see...
        if let Some(steam_path) = steam_path() {
            let registry_vdf = steam_path.parent().map(|p| p.join("registry.vdf"));
            if let Some(ref vdf_path) = registry_vdf {
                if vdf_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(vdf_path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("\"ActiveUser\"") {
                                let parts: Vec<&str> = trimmed.split('"').collect();
                                if parts.len() >= 4 {
                                    if let Ok(user_id) = parts[3].parse::<u32>() {
                                        if user_id != 0 {
                                            trace!(
                                                "Found active Steam user ID from registry.vdf: {}",
                                                user_id
                                            );
                                            return Some(user_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let userdata_path = steam_path.join("userdata");
            if userdata_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&userdata_path) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            if let Some(name) = entry.file_name().to_str() {
                                if let Ok(user_id) = name.parse::<u32>() {
                                    if user_id != 0 {
                                        trace!(
                                            "Found possibly active Steam user ID from userdata directory: {}",
                                            user_id
                                        );
                                        return Some(user_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn steam_running() -> bool {
    use sysinfo::System;

    let mut system = System::new_all();
    system.refresh_all();
    if system.processes().is_empty() {
        warn!("Failed to get process list to check for Steam process");
        return false;
    }

    for process in system.processes().values() {
        #[cfg(target_os = "windows")]
        {
            if process.name().to_str().unwrap_or_default() == "steam.exe" {
                return true;
            }
        }
        #[cfg(target_os = "linux")]
        {
            if process.name().to_str().unwrap_or_default() == "steam" {
                return true;
            }
        }
        #[cfg(target_os = "macos")]
        {
            if process.name().to_str().unwrap_or_default() == "steam_osx" {
                return true;
            }
        }
    }
    false
}

pub fn get_shortcuts_path(steam_path: &PathBuf, steam_active_user_id: u32) -> Option<PathBuf> {
    let joined_path: PathBuf = steam_path
        .clone()
        .join("userdata")
        .join(steam_active_user_id.to_string())
        .join("config/shortcuts.vdf");

    if joined_path.exists() {
        Some(joined_path)
    } else {
        None
    }
}

pub fn shortcuts_has_sisr_marker(shortcuts_path: &PathBuf) -> u32 {
    let shortcuts = open_shortcuts_vdf(shortcuts_path);
    trace!("Parsed shortcuts.vdf: {:?}", shortcuts);
    let running_executable_path = std::env::current_exe().unwrap_or_default();
    let running_path_str = running_executable_path
        .to_str()
        .unwrap_or_default()
        .to_lowercase();
    debug!("Current running executable path: {}", running_path_str);
    if let Some(shortcuts_array) = shortcuts.as_object() {
        for (_key, shortcut) in shortcuts_array {
            let Some(path) = shortcut.get("exe") else {
                continue;
            };
            let Some(args) = shortcut.get("LaunchOptions") else {
                continue;
            };
            let Some(path_str) = path.as_str() else {
                continue;
            };
            let Some(args_str) = args.as_str() else {
                continue;
            };
            trace!("Checking shortcut - Path: {}, Args: {}", path_str, args_str);
            if path_str
                .to_lowercase()
                .replace("\\", "/")
                .contains(&running_path_str.to_lowercase().replace("\\", "/"))
                && args_str.to_lowercase().contains("--marker")
            {
                let app_id = shortcut.get("appid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                return app_id;
            }
        }
    }
    0
}

pub async fn create_sisr_marker_shortcut() -> anyhow::Result<u32> {
    let mut payload = format!(
        "var SISR_PATH = `{}`;\n",
        std::env::current_exe()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .replace("\\", "/")
    ) + str::from_utf8(cef_debug::payloads::CREATE_MARKER_SHORTCUT)
        .expect("Failed to convert create marker shortcut payload to string");
    match cef_debug::inject::inject("SharedJSContext", &payload).await {
        Ok(result) => {
            debug!("Create SISR marker shortcut result: {}", result);
            let app_id: u32 = result.parse().unwrap_or(0);
            if app_id != 0 {
                Ok(app_id)
            } else {
                Err(anyhow::anyhow!(
                    "Failed to create SISR marker shortcut, invalid App ID returned"
                ))
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create SISR marker shortcut: {}",
            e
        )),
    }
}
