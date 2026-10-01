//! Theme constants matching the design specifications

use crate::platform::glazewm::BorderColors;
use iced::Color;

// Window dimensions
pub const WINDOW_WIDTH: f32 = 600.0;
pub const WINDOW_HEIGHT: f32 = 520.0;
pub const WINDOW_RADIUS: f32 = 18.0;

// Panel padding
pub const PANEL_PADDING: u16 = 22;

// Search bar
pub const SEARCH_RADIUS: f32 = 12.0;
pub const SEARCH_FONT_SIZE: f32 = 15.0;

// App list
pub const APP_LIST_TOP_MARGIN: u16 = 18;
pub const APP_ITEM_SPACING: u16 = 6;
pub const APP_ITEM_RADIUS: f32 = 10.0;
pub const APP_ITEM_PADDING_V: f32 = 10.0;
pub const APP_ITEM_PADDING_H: f32 = 14.0;

// App list item
pub const APP_ICON_SIZE: f32 = 32.0;
pub const APP_ICON_TEXT_GAP: u16 = 16;
pub const APP_NAME_FONT_SIZE: f32 = 14.0;

// Selection colors
pub const SELECTED_BG: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.18);
pub const HOVER_BG: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.10);

// Panel colors (used when GlazeWM colors aren't available)
pub const PANEL_BG: (f32, f32, f32, f32) = (0.24, 0.28, 0.36, 0.85);
pub const WINDOW_BORDER: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.1);

// Alpha applied to GlazeWM colors
pub const GLAZEWM_SELECTED_ALPHA: f32 = 0.30;
pub const GLAZEWM_PANEL_BG_ALPHA: f32 = PANEL_BG.3;

/// Colors that follow the GlazeWM theme
#[derive(Debug, Clone, Copy)]
pub struct Colors {
    /// Window outline
    pub border: Color,
    /// Panel background
    pub background: Color,
    /// Selected app row
    pub selected: Color,
}

impl Colors {
    /// Focused border color -> window outline and selected row;
    /// other-windows border color -> panel background.
    /// Each falls back to the built-in color when GlazeWM doesn't set it.
    pub fn from_glazewm(glazewm: &BorderColors) -> Self {
        let rgba = |(r, g, b, a): (f32, f32, f32, f32)| Color::from_rgba(r, g, b, a);

        Self {
            border: glazewm.focused.unwrap_or(rgba(WINDOW_BORDER)),
            background: glazewm
                .other
                .map(|c| Color { a: GLAZEWM_PANEL_BG_ALPHA, ..c })
                .unwrap_or(rgba(PANEL_BG)),
            selected: glazewm
                .focused
                .map(|c| Color { a: GLAZEWM_SELECTED_ALPHA, ..c })
                .unwrap_or(rgba(SELECTED_BG)),
        }
    }
}
