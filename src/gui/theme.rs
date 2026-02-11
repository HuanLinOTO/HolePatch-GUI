use gpui::*;

/// Color palette for the app
pub struct Theme;

impl Theme {
    // Background colors
    pub fn bg_primary() -> Hsla {
        hsla(0.63, 0.15, 0.12, 1.0)
    }

    pub fn bg_secondary() -> Hsla {
        hsla(0.63, 0.12, 0.16, 1.0)
    }

    pub fn bg_tertiary() -> Hsla {
        hsla(0.63, 0.10, 0.20, 1.0)
    }

    pub fn bg_input() -> Hsla {
        hsla(0.63, 0.08, 0.14, 1.0)
    }

    // Text colors
    pub fn text_primary() -> Hsla {
        hsla(0.0, 0.0, 0.92, 1.0)
    }

    pub fn text_secondary() -> Hsla {
        hsla(0.0, 0.0, 0.60, 1.0)
    }

    pub fn text_placeholder() -> Hsla {
        hsla(0.0, 0.0, 0.40, 1.0)
    }

    // Accent colors
    pub fn accent() -> Hsla {
        hsla(0.55, 0.65, 0.55, 1.0)
    }

    pub fn accent_hover() -> Hsla {
        hsla(0.55, 0.65, 0.65, 1.0)
    }

    // Status colors
    pub fn success() -> Hsla {
        hsla(0.35, 0.70, 0.50, 1.0)
    }

    pub fn warning() -> Hsla {
        hsla(0.10, 0.80, 0.55, 1.0)
    }

    pub fn error() -> Hsla {
        hsla(0.0, 0.70, 0.55, 1.0)
    }

    pub fn danger() -> Hsla {
        hsla(0.0, 0.65, 0.50, 1.0)
    }

    pub fn danger_hover() -> Hsla {
        hsla(0.0, 0.65, 0.60, 1.0)
    }

    // Border
    pub fn border() -> Hsla {
        hsla(0.63, 0.10, 0.25, 1.0)
    }

    pub fn border_focused() -> Hsla {
        hsla(0.55, 0.65, 0.55, 1.0)
    }

    // Log level colors
    pub fn log_debug() -> Hsla {
        hsla(0.0, 0.0, 0.50, 1.0)
    }

    pub fn log_info() -> Hsla {
        hsla(0.55, 0.50, 0.65, 1.0)
    }

    pub fn log_warn() -> Hsla {
        hsla(0.10, 0.80, 0.60, 1.0)
    }

    pub fn log_error() -> Hsla {
        hsla(0.0, 0.70, 0.60, 1.0)
    }
}
