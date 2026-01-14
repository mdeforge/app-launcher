//! Windows wallpaper retrieval

use std::path::PathBuf;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

/// Get the current desktop wallpaper path
pub fn get_wallpaper_path() -> Result<PathBuf, String> {
    const MAX_PATH: usize = 260;
    let mut buffer: Vec<u16> = vec![0u16; MAX_PATH];

    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .map_err(|e| e.to_string())?;
    }

    // Find null terminator and convert to String
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..len]);

    if path.is_empty() {
        // Try fallback: TranscodedWallpaper for Spotlight users
        if let Some(appdata) = dirs::config_dir() {
            let transcoded = appdata.join(r"Microsoft\Windows\Themes\TranscodedWallpaper");
            if transcoded.exists() {
                return Ok(transcoded);
            }
        }
        return Err("No wallpaper found".to_string());
    }

    Ok(PathBuf::from(path))
}
