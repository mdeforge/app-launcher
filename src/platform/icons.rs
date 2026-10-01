//! Windows icon extraction using the Shell's image factory

use crate::discovery::IconData;
use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
};

/// Initializes COM on the current thread for its lifetime.
/// The Shell icon APIs need COM; call `ComGuard::new()` before `extract_icon`.
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> Self {
        // S_OK and S_FALSE both need a matching CoUninitialize;
        // a failure (e.g. RPC_E_CHANGED_MODE) does not.
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
        Self { initialized }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}

/// Extract the icon Explorer shows for a path: an exe, a .lnk, a document,
/// or a `shell:AppsFolder\<AppID>` Store app
pub fn extract_icon(path: &str, size: u32) -> Option<IconData> {
    let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(path_wide.as_ptr()), None).ok()?;
        let size_px = SIZE { cx: size as i32, cy: size as i32 };
        let hbitmap = factory.GetImage(size_px, SIIGBF_ICONONLY).ok()?;

        let pixels = bitmap_rgba(hbitmap);
        let _ = DeleteObject(hbitmap);
        let (width, height, rgba) = pixels?;

        // Skip icons that came back fully transparent
        if rgba.chunks_exact(4).all(|px| px[3] == 0) {
            return None;
        }

        Some(IconData::new(width, height, rgba))
    }
}

/// Read a 32-bit bitmap as straight-alpha RGBA (top-down)
unsafe fn bitmap_rgba(hbitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut bm = BITMAP::default();
    let bm_size = std::mem::size_of::<BITMAP>() as i32;
    if GetObjectW(hbitmap, bm_size, Some(&mut bm as *mut _ as *mut _)) == 0 {
        return None;
    }

    let width = bm.bmWidth as u32;
    let height = bm.bmHeight.unsigned_abs();
    if width == 0 || height == 0 {
        return None;
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];

    let hdc = GetDC(None);
    let lines = GetDIBits(
        hdc,
        hbitmap,
        0,
        height,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, hdc);

    if lines == 0 {
        return None;
    }

    // Shell bitmaps are usually premultiplied BGRA, but some are already straight.
    // A color above its alpha is impossible when premultiplied, so that marks straight.
    let premultiplied = pixels
        .chunks_exact(4)
        .all(|px| px[..3].iter().all(|&c| c <= px[3]));

    // BGRA -> straight-alpha RGBA, which is what iced expects
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
        let a = px[3] as u32;
        if premultiplied && a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }

    Some((width, height, pixels))
}

