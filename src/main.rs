//! Application Launcher for Windows with GlazeWM integration
//! Runs as a resident daemon - GlazeWM sends IPC commands to toggle visibility
//!
//! Usage:
//!   - First launch: starts the daemon
//!   - Subsequent launches: sends "toggle" command to existing instance

// This prevents the console window from appearing
#![windows_subsystem = "windows"]

mod app;
mod discovery;
mod ipc;
mod platform;
mod search;
mod ui;

use app::Launcher;
use iced::{window, Size};
use ui::theme::{WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() -> iced::Result {
    // Try to acquire single-instance mutex
    let _mutex = match single_instance_mutex() {
        Some(m) => m,
        None => {
            // Another instance is running - send toggle and exit
            ipc::send_command("toggle");
            return Ok(());
        }
    };

    // We're the primary instance - start the daemon

    // Hide console window (backup, #![windows_subsystem] should handle this)
    #[cfg(windows)]
    hide_console_window();

    // Start the IPC server in background thread
    let ipc_rx = ipc::start_ipc_server();

    // Store the receiver in a static for the subscription to access
    IPC_RECEIVER.set(std::sync::Mutex::new(Some(ipc_rx))).ok();

    // Window settings: transparent, no decorations, centered
    let window_settings = window::Settings {
        size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        position: window::Position::Centered,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        visible: true,
        resizable: false,
        ..Default::default()
    };

    iced::application("App Launcher", Launcher::update, Launcher::view)
        .window(window_settings)
        .subscription(Launcher::subscription)
        .theme(Launcher::theme)
        .run_with(Launcher::new)
}

/// Global IPC receiver (set once at startup)
static IPC_RECEIVER: std::sync::OnceLock<std::sync::Mutex<Option<std::sync::mpsc::Receiver<ipc::IpcCommand>>>> =
    std::sync::OnceLock::new();

/// Get the IPC receiver (called from subscription)
pub fn take_ipc_receiver() -> Option<std::sync::mpsc::Receiver<ipc::IpcCommand>> {
    IPC_RECEIVER
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|mut guard| guard.take())
}

/// Create a named mutex for single instance enforcement
fn single_instance_mutex() -> Option<MutexGuard> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::System::Threading::CreateMutexW;

    let mutex_name: Vec<u16> = "Global\\AppLauncherMutex\0"
        .encode_utf16()
        .collect();

    unsafe {
        let handle = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())).ok()?;

        // Check if mutex already existed
        if GetLastError().0 == 183 {
            // ERROR_ALREADY_EXISTS
            return None;
        }

        Some(MutexGuard(handle))
    }
}

/// RAII guard for mutex handle
struct MutexGuard(windows::Win32::Foundation::HANDLE);

impl Drop for MutexGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Hide the console window on Windows
#[cfg(windows)]
fn hide_console_window() {
    use windows::Win32::System::Console::GetConsoleWindow;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    unsafe {
        let console = GetConsoleWindow();
        if !console.is_invalid() {
            let _ = ShowWindow(console, SW_HIDE);
        }
    }
}
