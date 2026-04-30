use crate::color::blend;
use crate::color::is_light;
use crate::terminal_palette::best_color;
use crate::terminal_palette::default_bg;
use ratatui::style::Color;
use ratatui::style::Style;

const LIGHT_USER_ACCENT: (u8, u8, u8) = (32, 100, 135);
const DARK_USER_ACCENT: (u8, u8, u8) = (88, 178, 205);

pub fn user_message_style() -> Style {
    user_message_style_for(default_bg())
}

pub fn user_message_marker_style() -> Style {
    user_message_marker_style_for(default_bg())
}

pub fn proposed_plan_style() -> Style {
    proposed_plan_style_for(default_bg())
}

/// Returns the style for a user-authored message using the provided terminal background.
pub fn user_message_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(user_message_bg(bg)),
        None => Style::default(),
    }
}

pub fn user_message_marker_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().fg(user_message_accent(bg)),
        None => Style::default().fg(Color::Cyan),
    }
}

pub fn proposed_plan_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(proposed_plan_bg(bg)),
        None => Style::default(),
    }
}

#[allow(clippy::disallowed_methods)]
pub fn user_message_bg(terminal_bg: (u8, u8, u8)) -> Color {
    best_color(user_message_bg_rgb(terminal_bg))
}

fn user_message_bg_rgb(terminal_bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let (top, alpha) = if is_light(terminal_bg) {
        (LIGHT_USER_ACCENT, 0.10)
    } else {
        (DARK_USER_ACCENT, 0.18)
    };
    blend(top, terminal_bg, alpha)
}

#[allow(clippy::disallowed_methods)]
pub fn user_message_accent(terminal_bg: (u8, u8, u8)) -> Color {
    let accent = if is_light(terminal_bg) {
        LIGHT_USER_ACCENT
    } else {
        DARK_USER_ACCENT
    };
    best_color(accent)
}

#[allow(clippy::disallowed_methods)]
pub fn proposed_plan_bg(terminal_bg: (u8, u8, u8)) -> Color {
    best_color(proposed_plan_bg_rgb(terminal_bg))
}

fn proposed_plan_bg_rgb(terminal_bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let (top, alpha) = if is_light(terminal_bg) {
        ((0, 0, 0), 0.04)
    } else {
        ((255, 255, 255), 0.12)
    };
    blend(top, terminal_bg, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_background_remains_subtle_but_stronger_than_plan_surface() {
        let light_bg = (245, 245, 245);
        let dark_bg = (18, 18, 18);

        assert_ne!(user_message_bg_rgb(light_bg), proposed_plan_bg_rgb(light_bg));
        assert_ne!(user_message_bg_rgb(dark_bg), proposed_plan_bg_rgb(dark_bg));
    }

    #[test]
    fn user_message_marker_has_unknown_background_fallback() {
        let marker = user_message_marker_style_for(None);

        assert_eq!(marker.fg, Some(Color::Cyan));
        assert_eq!(marker.bg, None);
    }
}
