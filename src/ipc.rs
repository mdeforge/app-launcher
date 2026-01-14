//! Named Pipe IPC for daemon-style operation
//! Allows GlazeWM to send toggle/show/hide commands

use std::sync::mpsc;
use std::thread;
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

/// The named pipe path for the launcher
pub const PIPE_NAME: &str = r"\\.\pipe\app-launcher-ipc";

/// Commands that can be sent via IPC
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    Toggle,
    Show,
    Hide,
    Quit,
}

impl IpcCommand {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "toggle" => Some(IpcCommand::Toggle),
            "show" => Some(IpcCommand::Show),
            "hide" => Some(IpcCommand::Hide),
            "quit" | "exit" => Some(IpcCommand::Quit),
            "ping" => None, // Ping is just for checking if server is alive
            _ => None,
        }
    }
}

/// Start the IPC server in a background thread
/// Returns a receiver for incoming commands
pub fn start_ipc_server() -> mpsc::Receiver<IpcCommand> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            if let Some(cmd) = wait_for_command() {
                if tx.send(cmd.clone()).is_err() {
                    // Main thread dropped, exit
                    break;
                }
                if cmd == IpcCommand::Quit {
                    break;
                }
            }
        }
    });

    rx
}

/// Wait for a single command on the named pipe
fn wait_for_command() -> Option<IpcCommand> {
    unsafe {
        // Create the named pipe
        let pipe_name: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

        let pipe = CreateNamedPipeW(
            PCWSTR(pipe_name.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            512,
            512,
            0,
            None,
        );

        if pipe.is_invalid() {
            eprintln!("Failed to create named pipe");
            return None;
        }

        // Wait for a client to connect
        let connected = ConnectNamedPipe(pipe, None);
        if connected.is_err() {
            // Could be ERROR_PIPE_CONNECTED which is actually ok
            let err = windows::Win32::Foundation::GetLastError();
            if err.0 != 535 {
                // ERROR_PIPE_CONNECTED
                CloseHandle(pipe).ok();
                return None;
            }
        }

        // Read the command
        let mut buffer = [0u8; 256];
        let mut bytes_read = 0u32;

        let read_ok = ReadFile(pipe, Some(&mut buffer), Some(&mut bytes_read), None);

        let cmd = if read_ok.is_ok() && bytes_read > 0 {
            let msg = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
            IpcCommand::from_str(&msg)
        } else {
            None
        };

        // Disconnect and close
        DisconnectNamedPipe(pipe).ok();
        CloseHandle(pipe).ok();

        cmd
    }
}

/// Send a command to the running launcher instance
/// Returns true if successful
pub fn send_command(cmd: &str) -> bool {
    unsafe {
        let pipe_name: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

        // Try to open the existing pipe
        let pipe = CreateFileW(
            PCWSTR(pipe_name.as_ptr()),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        );

        if let Ok(handle) = pipe {
            let cmd_bytes = cmd.as_bytes();
            let mut bytes_written = 0u32;

            let write_ok = WriteFile(handle, Some(cmd_bytes), Some(&mut bytes_written), None);

            FlushFileBuffers(handle).ok();
            CloseHandle(handle).ok();

            write_ok.is_ok()
        } else {
            false
        }
    }
}
