//! Palette + small paint helpers. Modern-minimal tracker palette mirroring
//! the TUI's default theme: low-saturation greens/cyans on near-black,
//! magenta accents.

use eframe::egui::Color32;

pub const BG: Color32 = Color32::from_rgb(0x0e, 0x12, 0x14);
pub const PANEL: Color32 = Color32::from_rgb(0x12, 0x17, 0x1a);
pub const TRACK: Color32 = Color32::from_rgb(0x1e, 0x28, 0x2b);
pub const CURRENT_ROW_BG: Color32 = Color32::from_rgb(0x1a, 0x26, 0x24);
pub const FG: Color32 = Color32::from_rgb(0xcf, 0xe6, 0xd8);
pub const DIM: Color32 = Color32::from_rgb(0x5a, 0x6e, 0x66);
pub const GREEN: Color32 = Color32::from_rgb(0x66, 0xd9, 0xa5);
pub const CYAN: Color32 = Color32::from_rgb(0x5a, 0xc8, 0xc8);
pub const MAGENTA: Color32 = Color32::from_rgb(0xc6, 0x78, 0xa8);

// Pattern cell tokens, mirroring the TUI theme's note/inst/vol/effect colors.
pub const NOTE: Color32 = FG;
pub const INSTRUMENT: Color32 = CYAN;
pub const VOLUME: Color32 = GREEN;
pub const EFFECT: Color32 = MAGENTA;

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

pub fn fmt_mmss(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_mmss_formats_minutes_and_seconds() {
        assert_eq!(fmt_mmss(0.0), "00:00");
        assert_eq!(fmt_mmss(59.9), "00:59");
        assert_eq!(fmt_mmss(61.0), "01:01");
        assert_eq!(fmt_mmss(600.0), "10:00");
        assert_eq!(fmt_mmss(-3.0), "00:00");
    }

    #[test]
    fn lerp_color_endpoints_and_midpoint() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        assert_eq!(lerp_color(a, b, 0.5), Color32::from_rgb(100, 50, 25));
    }
}
