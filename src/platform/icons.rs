//! Windows icon extraction using Shell API

use crate::discovery::IconData;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    CreateCompatibleDC, DeleteDC, SelectObject, GetObjectW, BITMAP,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO, HICON};

/// Extract icon from a file path (exe, lnk, etc.)
pub fn extract_icon(path: &Path, size: u32) -> Option<IconData> {
    // Convert path to wide string
    let path_wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        // Extract icon using Shell API
        let mut large_icon: HICON = HICON::default();
        let mut small_icon: HICON = HICON::default();

        let count = ExtractIconExW(
            PCWSTR(path_wide.as_ptr()),
            0,
            Some(&mut large_icon),
            Some(&mut small_icon),
            1,
        );

        if count == 0 {
            return None;
        }

        // Use the large icon (better quality)
        let icon = if large_icon.0 as usize != 0 {
            large_icon
        } else if small_icon.0 as usize != 0 {
            small_icon
        } else {
            return None;
        };

        // Get icon info to access the bitmap
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(icon, &mut icon_info).is_err() {
            let _ = DestroyIcon(icon);
            return None;
        }

        // Get bitmap info
        let hbm_color = icon_info.hbmColor;
        if hbm_color.is_invalid() {
            if !icon_info.hbmMask.is_invalid() {
                let _ = DeleteObject(icon_info.hbmMask);
            }
            let _ = DestroyIcon(icon);
            return None;
        }

        let mut bm = BITMAP::default();
        let bm_size = std::mem::size_of::<BITMAP>() as i32;
        if GetObjectW(hbm_color, bm_size, Some(&mut bm as *mut _ as *mut _)) == 0 {
            let _ = DeleteObject(hbm_color);
            if !icon_info.hbmMask.is_invalid() {
                let _ = DeleteObject(icon_info.hbmMask);
            }
            let _ = DestroyIcon(icon);
            return None;
        }

        let width = bm.bmWidth as u32;
        let height = bm.bmHeight.unsigned_abs();

        // Create DC for bitmap operations
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            let _ = DeleteObject(hbm_color);
            if !icon_info.hbmMask.is_invalid() {
                let _ = DeleteObject(icon_info.hbmMask);
            }
            let _ = DestroyIcon(icon);
            return None;
        }

        let old_bitmap = SelectObject(hdc, hbm_color);

        // Prepare bitmap info for GetDIBits
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // Top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        // Allocate buffer for pixel data
        let buffer_size = (width * height * 4) as usize;
        let mut pixels: Vec<u8> = vec![0; buffer_size];

        let result = GetDIBits(
            hdc,
            hbm_color,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup
        SelectObject(hdc, old_bitmap);
        let _ = DeleteDC(hdc);
        let _ = DeleteObject(hbm_color);
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask);
        }
        if large_icon.0 as usize != 0 {
            let _ = DestroyIcon(large_icon);
        }
        if small_icon.0 as usize != 0 && small_icon != large_icon {
            let _ = DestroyIcon(small_icon);
        }

        if result == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }

        // Resize if needed (simple nearest-neighbor for speed)
        let rgba = if width != size || height != size {
            resize_rgba(&pixels, width, height, size, size)
        } else {
            pixels
        };

        Some(IconData {
            width: size,
            height: size,
            rgba,
        })
    }
}

/// Simple nearest-neighbor resize for RGBA images
fn resize_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];

    for y in 0..dst_h {
        for x in 0..dst_w {
            let src_x = (x * src_w / dst_w).min(src_w - 1);
            let src_y = (y * src_h / dst_h).min(src_h - 1);

            let src_idx = ((src_y * src_w + src_x) * 4) as usize;
            let dst_idx = ((y * dst_w + x) * 4) as usize;

            if src_idx + 3 < src.len() && dst_idx + 3 < dst.len() {
                dst[dst_idx] = src[src_idx];
                dst[dst_idx + 1] = src[src_idx + 1];
                dst[dst_idx + 2] = src[src_idx + 2];
                dst[dst_idx + 3] = src[src_idx + 3];
            }
        }
    }

    dst
}
