//! System tray icon: left-click opens the launcher, right-click shows a menu

use crate::ipc::IpcCommand;
use iced::Color;
use std::sync::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

/// Tray icon size in pixels (Windows scales it to the tray's DPI)
const ICON_SIZE: u32 = 32;

/// Start the tray icon on its own thread.
/// Tray clicks and menu picks are sent to the launcher as IPC commands.
pub fn start_tray(tx: Sender<IpcCommand>, accent: Color) {
    std::thread::spawn(move || run_tray(tx, accent));
}

/// Create the tray icon and run the Win32 message loop it needs.
/// tray-icon requires the icon to be created on the thread that pumps its messages.
fn run_tray(tx: Sender<IpcCommand>, accent: Color) {
    let open = MenuItem::new("Open", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let Ok(menu) = Menu::with_items(&[&open, &PredefinedMenuItem::separator(), &quit]) else {
        return;
    };
    let Ok(icon) = Icon::from_rgba(draw_icon(accent), ICON_SIZE, ICON_SIZE) else {
        return;
    };

    let Ok(tray) = TrayIconBuilder::new()
        .with_tooltip("App Launcher")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
    else {
        return;
    };

    let mut msg = MSG::default();
    // GetMessageW returns 0 on WM_QUIT and -1 on error
    while unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Dispatching may have produced tray or menu events
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // Show rather than toggle: the click itself takes focus from the
                // launcher, which hides it before this event arrives
                let _ = tx.send(IpcCommand::Show);
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id() == open.id() {
                let _ = tx.send(IpcCommand::Show);
            } else if event.id() == quit.id() {
                // Remove the icon before the app exits so no stale icon is left behind
                drop(tray);
                let _ = tx.send(IpcCommand::Quit);
                return;
            }
        }
    }
}

/// Draw a magnifying glass in the accent color as RGBA pixels
fn draw_icon(accent: Color) -> Vec<u8> {
    let [r, g, b, a] = accent.into_rgba8();
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            // Signed distances (negative = inside) to the lens ring and the handle
            let lens = ((p.0 - 13.0).hypot(p.1 - 13.0) - 8.0).abs() - 2.0;
            let handle = segment_distance(p, (19.0, 19.0), (28.0, 28.0)) - 2.5;
            // Antialiased coverage of the nearer shape
            let coverage = (0.5 - lens.min(handle)).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[r, g, b, (a as f32 * coverage).round() as u8]);
        }
    }

    rgba
}

/// Distance from point `p` to the line segment `a`-`b`
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / (abx * abx + aby * aby)).clamp(0.0, 1.0);
    (p.0 - (a.0 + t * abx)).hypot(p.1 - (a.1 + t * aby))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(rgba: &[u8], x: u32, y: u32) -> u8 {
        rgba[((y * ICON_SIZE + x) * 4 + 3) as usize]
    }

    #[test]
    fn icon_is_a_magnifying_glass() {
        let rgba = draw_icon(Color::WHITE);
        assert_eq!(rgba.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        assert_eq!(alpha_at(&rgba, 13, 13), 0, "lens center is see-through");
        assert_eq!(alpha_at(&rgba, 13, 5), 255, "lens ring is opaque");
        assert_eq!(alpha_at(&rgba, 24, 24), 255, "handle is opaque");
        assert_eq!(alpha_at(&rgba, 0, 31), 0, "corner away from the glass is empty");
    }

    #[test]
    fn segment_distance_clamps_to_endpoints() {
        assert_eq!(segment_distance((0.0, 0.0), (0.0, 0.0), (10.0, 0.0)), 0.0);
        assert_eq!(segment_distance((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)), 3.0);
        assert_eq!(segment_distance((13.0, 0.0), (0.0, 0.0), (10.0, 0.0)), 3.0);
    }
}
