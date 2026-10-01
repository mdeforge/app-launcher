//! Windows application discovery from Start Menu

use serde::{Deserialize, Serialize};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Represents icon pixel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Represents a discovered application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    /// Display name
    pub name: String,
    /// Path to executable
    pub exec_path: Option<String>,
    /// Icon data (RGBA pixels)
    #[serde(skip)]
    pub icon_data: Option<IconData>,
}

/// Discover applications from Start Menu
pub async fn discover_apps() -> Vec<AppEntry> {
    // Run discovery in a separate blocking task
    match tokio::task::spawn_blocking(discover_apps_sync).await {
        Ok(apps) => apps,
        Err(_) => Vec::new(),
    }
}

/// Synchronous app discovery
fn discover_apps_sync() -> Vec<AppEntry> {
    // Set a custom panic hook to suppress lnk library panics
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {
        // Silently ignore panics during app discovery
    }));

    let result = discover_apps_inner();

    // Restore the previous panic hook
    panic::set_hook(prev_hook);

    result
}

fn discover_apps_inner() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // Start Menu locations
    let locations = [
        // System-wide Start Menu
        PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"),
        // User Start Menu
        dirs::data_dir()
            .map(|p| p.join(r"Microsoft\Windows\Start Menu\Programs"))
            .unwrap_or_default(),
        // Desktop shortcuts (user)
        dirs::desktop_dir().unwrap_or_default(),
        // Desktop shortcuts (public)
        PathBuf::from(r"C:\Users\Public\Desktop"),
    ];

    for location in locations {
        if !location.exists() {
            continue;
        }

        for entry in WalkDir::new(&location)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(false)
            })
        {
            if let Some(app) = parse_shortcut_safe(entry.path()) {
                // Deduplicate by name
                if !seen_names.contains(&app.name.to_lowercase()) {
                    seen_names.insert(app.name.to_lowercase());
                    apps.push(app);
                }
            }
        }
    }

    // Also discover Windows Store / MSIX apps (Teams, etc.)
    discover_uwp_apps(&mut apps, &mut seen_names);

    // Sort alphabetically
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Extract icons for all apps
    for app in &mut apps {
        if let Some(ref exec_path) = app.exec_path {
            let path = std::path::Path::new(exec_path);
            // Try to extract icon (32x32 for app list)
            if let Some(icon) = crate::platform::icons::extract_icon(path, 32) {
                app.icon_data = Some(icon);
            }
        }
    }

    apps
}

/// Parse a .lnk shortcut file safely (catches panics)
fn parse_shortcut_safe(path: &std::path::Path) -> Option<AppEntry> {
    // Get app name from filename first (this is safe)
    let mut name = path.file_stem()?.to_string_lossy().to_string();

    // Remove duplicate suffixes like " (2)", " (3)", " - Copy", etc.
    name = normalize_app_name(&name);

    // Skip certain system entries
    let name_lower = name.to_lowercase();
    if name_lower.contains("uninstall")
        || name_lower.contains("readme")
        || name_lower.contains("help")
        || name_lower.contains("documentation")
        || name_lower.contains("website")
        || name_lower.contains("manual")
    {
        return None;
    }

    // Try to parse the .lnk file for the target path
    // Spawn a separate thread to catch panics (tokio doesn't propagate catch_unwind properly)
    let path_owned = path.to_owned();
    let exec_path = std::thread::spawn(move || {
        panic::catch_unwind(AssertUnwindSafe(|| {
            parse_lnk_target(&path_owned)
        }))
        .ok()
        .flatten()
    })
    .join()
    .ok()
    .flatten();

    // If we got a target path, use it; otherwise use the .lnk path itself
    let final_exec_path = exec_path.or_else(|| Some(path.to_string_lossy().to_string()));

    Some(AppEntry {
        name,
        exec_path: final_exec_path,
        icon_data: None,
    })
}

/// Normalize app name by removing duplicate suffixes like " (2)", " (3)", " - Copy"
fn normalize_app_name(name: &str) -> String {
    let mut result = name.to_string();

    // Remove patterns like " (2)", " (3)", " (4)", etc.
    let re_numbered = regex::Regex::new(r"\s*\(\d+\)\s*$").unwrap();
    result = re_numbered.replace(&result, "").to_string();

    // Remove patterns like " - Copy", " - Shortcut"
    let re_copy = regex::Regex::new(r"\s*-\s*(Copy|Shortcut)\s*$").unwrap();
    result = re_copy.replace(&result, "").to_string();

    result.trim().to_string()
}

/// Try to extract target path from .lnk file
fn parse_lnk_target(path: &std::path::Path) -> Option<String> {
    // Some .lnk files cause the lnk library to panic internally
    // We use a thread with catch_unwind to handle this gracefully
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let lnk = lnk::ShellLink::open(path).ok()?;
        lnk.link_info()
            .as_ref()
            .and_then(|li| li.local_base_path().clone())
    }));

    result.ok().flatten()
}

/// Discover UWP/MSIX apps (Teams, Store apps, etc.)
/// Uses PowerShell Get-StartApps to find all registered apps
fn discover_uwp_apps(apps: &mut Vec<AppEntry>, seen_names: &mut std::collections::HashSet<String>) {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    // Use PowerShell to get all Start Apps (includes UWP, MSIX, and regular apps).
    // CREATE_NO_WINDOW: without it, Windows opens a terminal window for PowerShell
    // that steals focus, and the launcher hides itself on focus loss.
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-StartApps | ForEach-Object { $_.Name + '|' + $_.AppID }"
        ])
        .creation_flags(CREATE_NO_WINDOW.0)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim().to_string();
                    let app_id = parts[1].trim().to_string();
                    let name_lower = name.to_lowercase();

                    // Skip if already seen or if it's a system/uninstall entry
                    if seen_names.contains(&name_lower)
                        || name_lower.contains("uninstall")
                        || name_lower.contains("readme")
                        || name_lower.contains("help")
                        || name_lower.is_empty()
                    {
                        continue;
                    }

                    seen_names.insert(name_lower);

                    // Use shell:AppsFolder\AppID as the launch command
                    let exec_path = format!("shell:AppsFolder\\{}", app_id);

                    apps.push(AppEntry {
                        name,
                        exec_path: Some(exec_path),
                        icon_data: None,
                    });
                }
            }
        }
    }
}

/// Get cache file path
fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("al").join("apps_cache.json"))
}

/// Load apps from cache
pub fn load_cached_apps() -> Option<Vec<AppEntry>> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Cache apps to disk
pub async fn cache_apps(apps: &[AppEntry]) -> Result<(), std::io::Error> {
    let path = match cache_path() {
        Some(p) => p,
        None => return Ok(()),
    };

    // Create cache directory
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let data = serde_json::to_string(apps)?;
    tokio::fs::write(path, data).await?;

    Ok(())
}
