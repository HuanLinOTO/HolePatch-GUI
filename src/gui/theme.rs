use gpui::*;

/// Modern dark theme palette
/// Inspired by VS Code / Discord aesthetics — deep backgrounds, subtle borders, vibrant accents.
pub struct Theme;

impl Theme {
    // ── Background layers (darkest → lightest) ──────────────────────
    /// App background / deepest layer
    pub fn bg_primary() -> Hsla {
        hsla(0.63, 0.12, 0.10, 1.0)
    }

    /// Sidebar / panel background
    pub fn bg_secondary() -> Hsla {
        hsla(0.63, 0.10, 0.13, 1.0)
    }

    /// Cards, elevated surfaces
    pub fn bg_elevated() -> Hsla {
        hsla(0.63, 0.08, 0.16, 1.0)
    }

    /// Input fields
    pub fn bg_input() -> Hsla {
        hsla(0.63, 0.10, 0.09, 1.0)
    }

    /// Hover state for interactive surfaces
    pub fn bg_hover() -> Hsla {
        hsla(0.63, 0.10, 0.18, 1.0)
    }

    // ── Text ────────────────────────────────────────────────────────
    pub fn text_primary() -> Hsla {
        hsla(0.0, 0.0, 0.93, 1.0)
    }

    pub fn text_secondary() -> Hsla {
        hsla(0.0, 0.0, 0.55, 1.0)
    }

    pub fn text_muted() -> Hsla {
        hsla(0.0, 0.0, 0.38, 1.0)
    }

    // ── Accent (teal-cyan) ──────────────────────────────────────────
    pub fn accent() -> Hsla {
        hsla(0.48, 0.72, 0.56, 1.0)
    }

    pub fn accent_hover() -> Hsla {
        hsla(0.48, 0.72, 0.64, 1.0)
    }

    pub fn accent_muted() -> Hsla {
        hsla(0.48, 0.40, 0.25, 1.0)
    }

    // ── Status ──────────────────────────────────────────────────────
    pub fn success() -> Hsla {
        hsla(0.38, 0.68, 0.48, 1.0)
    }

    pub fn warning() -> Hsla {
        hsla(0.10, 0.85, 0.58, 1.0)
    }

    pub fn error() -> Hsla {
        hsla(0.0, 0.65, 0.55, 1.0)
    }

    pub fn danger() -> Hsla {
        hsla(0.0, 0.60, 0.48, 1.0)
    }

    pub fn danger_hover() -> Hsla {
        hsla(0.0, 0.60, 0.56, 1.0)
    }

    // ── Borders ─────────────────────────────────────────────────────
    pub fn border() -> Hsla {
        hsla(0.63, 0.06, 0.20, 1.0)
    }

    pub fn border_subtle() -> Hsla {
        hsla(0.63, 0.06, 0.16, 1.0)
    }

    pub fn border_focused() -> Hsla {
        hsla(0.48, 0.72, 0.56, 1.0)
    }

    // ── Log level colors ────────────────────────────────────────────
    pub fn log_debug() -> Hsla {
        hsla(0.0, 0.0, 0.45, 1.0)
    }

    pub fn log_info() -> Hsla {
        hsla(0.55, 0.50, 0.68, 1.0)
    }

    pub fn log_warn() -> Hsla {
        hsla(0.10, 0.85, 0.65, 1.0)
    }

    pub fn log_error() -> Hsla {
        hsla(0.0, 0.70, 0.65, 1.0)
    }

    // ── Helpers ─────────────────────────────────────────────────────
    pub fn white() -> Hsla {
        hsla(0.0, 0.0, 1.0, 1.0)
    }

    pub fn transparent() -> Hsla {
        hsla(0.0, 0.0, 0.0, 0.0)
    }

    /// Section label color — slightly brighter than text_secondary
    pub fn section_label() -> Hsla {
        hsla(0.48, 0.30, 0.55, 1.0)
    }
}
