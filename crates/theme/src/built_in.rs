use std::sync::LazyLock;

use gpui::FontStyle;
use gpui::FontWeight;
use gpui::HighlightStyle;
use gpui::Hsla;
use gpui::SharedString;
use gpui::WindowBackgroundAppearance;
use gpui::rgb;
use gpui::transparent_black;

use crate::AccentColors;
use crate::Appearance;
use crate::CursorColors;
use crate::StatusColors;
use crate::SyntaxTheme;
use crate::SystemColors;
use crate::Theme;
use crate::ThemeColors;
use crate::ThemeStyles;

/// Buffer and terminal line height, relative to the font size.
pub const BUFFER_LINE_HEIGHT: f32 = 1.618;
/// Opacity reduction applied to unnecessary (e.g. unused) code.
pub const UNNECESSARY_CODE_FADE: f32 = 0.3;

/// The only theme: One Dark with the floating-card chrome. Edit these constructors to change the look.
pub fn theme() -> &'static Theme {
    static THEME: LazyLock<Theme> = LazyLock::new(|| Theme {
        id: "one-dark".into(),
        name: SharedString::new_static("One Dark"),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: system_colors(),
            accents: accent_colors(),
            colors: theme_colors(),
            status: status_colors(),
            cursor_colors: cursor_colors(),
            syntax: syntax_theme(),
        },
    });
    &THEME
}

// One Dark palette: the only raw colour values of the theme. Everything below derives from these.
const DARK0: u32 = 0x282c34;
const DARK1: u32 = 0x3e4451;
const DARK2: u32 = 0x4b5263;
const DARK3: u32 = 0x5c6370;
const LIGHT0: u32 = 0xabb2bf;
const LIGHT1: u32 = 0xc8ccd4;
const LIGHT2: u32 = 0xdcdfe4;
const RED: u32 = 0xe06c75;
const ORANGE: u32 = 0xd19a66;
const YELLOW: u32 = 0xe5c07b;
const GREEN: u32 = 0x98c379;
const CYAN: u32 = 0x56b6c2;
const BLUE: u32 = 0x61afef;
const PURPLE: u32 = 0xc678dd;

// Roles: what each palette colour (or a formula of it) is used for.
/// Everything around the cards: gaps, title bar, status bar, tab bar, side docks.
const CHROME: u32 = shade(DARK0, -30);
/// Editor and terminal cards, darker than the chrome; the active tab joins its card.
const CARD: u32 = shade(CHROME, -33);
const POPUP: u32 = shade(DARK1, -30);
const BORDER: u32 = DARK1;
const HIGHLIGHT: u32 = DARK2;
const FAINT: u32 = DARK3;
const SELECTION: u32 = shade(DARK3, 50);
const TEXT: u32 = LIGHT0;
const TEXT_MUTED: u32 = shade(LIGHT0, -20);
const TEXT_DIM: u32 = shade(LIGHT0, -30);
const CODE_HINT: u32 = shade(DARK3, 20);
const COMMENT: u32 = DARK3;
const ACCENT: u32 = BLUE;
const IGNORED: u32 = shade(YELLOW, -40);

/// Terminal bright / dim variants of an ANSI colour.
const BRIGHT: i32 = 20;
const DIM: i32 = -30;

pub fn theme_colors() -> ThemeColors {
    let transparent = transparent_black();
    let scrollbar = color(HIGHLIGHT).alpha(0.5);
    let scrollbar_hover = color(HIGHLIGHT).alpha(0.67);
    let indent_guide = color(TEXT).alpha(0.1);
    let indent_guide_active = color(TEXT).alpha(0.18);
    ThemeColors {
        border: color(BORDER),
        border_variant: color(BORDER),
        border_focused: color(BORDER),
        border_selected: color(BORDER),
        border_transparent: transparent,
        border_disabled: color(FAINT),
        elevated_surface_background: color(POPUP),
        surface_background: color(CHROME),
        background: color(CHROME),
        element_background: color(BORDER),
        element_hover: color(BORDER).alpha(0.5),
        element_active: color(BORDER),
        element_selected: color(BORDER),
        element_selection_background: color(SELECTION).alpha(0.5),
        element_disabled: color(BORDER),
        ghost_element_background: transparent,
        ghost_element_hover: color(HIGHLIGHT),
        ghost_element_active: color(HIGHLIGHT),
        ghost_element_selected: color(FAINT),
        ghost_element_disabled: color(BORDER),
        text: color(TEXT),
        text_muted: color(TEXT_MUTED),
        text_placeholder: color(TEXT_MUTED),
        text_disabled: color(TEXT_MUTED),
        text_accent: color(ACCENT),
        icon: color(TEXT),
        icon_muted: color(TEXT_MUTED),
        icon_disabled: color(TEXT_MUTED),
        icon_placeholder: color(BORDER),
        icon_accent: color(ACCENT),
        status_bar_background: color(CHROME),
        title_bar_background: color(CHROME),
        title_bar_inactive_background: color(CHROME),
        toolbar_background: color(CARD),
        tab_bar_background: color(CHROME),
        tab_inactive_background: color(CHROME),
        tab_active_background: color(CARD),
        search_match_background: color(YELLOW).alpha(0.67),
        search_active_match_background: color(RED).alpha(0.67),
        panel_background: color(CHROME),
        panel_focused_border: color(ACCENT),
        panel_indent_guide: indent_guide,
        panel_indent_guide_hover: indent_guide_active,
        panel_indent_guide_active: indent_guide_active,
        panel_overlay_background: color(CHROME),
        // `element_hover` flattened onto the chrome: the overlay must stay opaque.
        panel_overlay_hover: color(mix(BORDER, CHROME, 50)),
        pane_focused_border: color(ACCENT),
        pane_group_border: color(BORDER),
        scrollbar_thumb_background: scrollbar,
        scrollbar_thumb_hover_background: scrollbar_hover,
        scrollbar_thumb_active_background: scrollbar_hover,
        scrollbar_thumb_border: transparent,
        scrollbar_track_background: color(CHROME),
        scrollbar_track_border: transparent,
        minimap_thumb_background: scrollbar,
        minimap_thumb_hover_background: scrollbar_hover,
        minimap_thumb_active_background: scrollbar_hover,
        minimap_thumb_border: transparent,
        editor_foreground: color(TEXT),
        editor_code_lens_foreground: None,
        editor_background: color(CARD),
        editor_gutter_background: color(CARD),
        editor_subheader_background: color(mix(BORDER, CHROME, 60)),
        editor_active_line_background: color(BORDER).alpha(0.5),
        editor_highlighted_line_background: color(HIGHLIGHT),
        editor_line_number: color(FAINT),
        editor_active_line_number: color(TEXT),
        editor_hover_line_number: color(LIGHT2),
        editor_invisible: color(TEXT_DIM),
        editor_wrap_guide: color(FAINT),
        editor_active_wrap_guide: color(FAINT),
        editor_indent_guide: indent_guide,
        editor_indent_guide_active: indent_guide_active,
        editor_document_highlight_read_background: color(HIGHLIGHT),
        editor_document_highlight_write_background: color(HIGHLIGHT),
        editor_document_highlight_bracket_background: color(HIGHLIGHT),
        editor_diff_hunk_added_background: color(GREEN).alpha(0.12),
        editor_diff_hunk_added_hollow_background: color(GREEN).alpha(0.06),
        editor_diff_hunk_added_hollow_border: color(GREEN).alpha(0.36),
        editor_diff_hunk_deleted_background: color(RED).alpha(0.12),
        editor_diff_hunk_deleted_hollow_background: color(RED).alpha(0.06),
        editor_diff_hunk_deleted_hollow_border: color(RED).alpha(0.36),
        terminal_background: color(CARD),
        terminal_foreground: color(TEXT),
        terminal_bright_foreground: color(LIGHT2),
        terminal_dim_foreground: color(shade(TEXT, DIM)),
        terminal_ansi_background: color(CARD),
        terminal_ansi_black: color(DARK1),
        terminal_ansi_bright_black: color(DARK3),
        terminal_ansi_dim_black: color(shade(DARK1, DIM)),
        terminal_ansi_red: color(RED),
        terminal_ansi_bright_red: color(shade(RED, BRIGHT)),
        terminal_ansi_dim_red: color(shade(RED, DIM)),
        terminal_ansi_green: color(GREEN),
        terminal_ansi_bright_green: color(shade(GREEN, BRIGHT)),
        terminal_ansi_dim_green: color(shade(GREEN, DIM)),
        terminal_ansi_yellow: color(YELLOW),
        terminal_ansi_bright_yellow: color(shade(YELLOW, BRIGHT)),
        terminal_ansi_dim_yellow: color(shade(YELLOW, DIM)),
        terminal_ansi_blue: color(BLUE),
        terminal_ansi_bright_blue: color(shade(BLUE, BRIGHT)),
        terminal_ansi_dim_blue: color(shade(BLUE, DIM)),
        terminal_ansi_magenta: color(PURPLE),
        terminal_ansi_bright_magenta: color(shade(PURPLE, BRIGHT)),
        terminal_ansi_dim_magenta: color(shade(PURPLE, DIM)),
        terminal_ansi_cyan: color(CYAN),
        terminal_ansi_bright_cyan: color(shade(CYAN, BRIGHT)),
        terminal_ansi_dim_cyan: color(shade(CYAN, DIM)),
        terminal_ansi_white: color(LIGHT1),
        terminal_ansi_bright_white: color(LIGHT2),
        terminal_ansi_dim_white: color(shade(LIGHT1, DIM)),
        link_text_hover: color(CYAN),
        version_control_added: color(GREEN),
        version_control_deleted: color(RED),
        version_control_modified: color(YELLOW),
        version_control_renamed: color(ORANGE),
        version_control_conflict: color(RED),
        version_control_ignored: color(IGNORED),
        version_control_word_added: color(shade(GREEN, -50)),
        version_control_word_deleted: color(shade(RED, -50)),
        version_control_conflict_marker_ours: color(GREEN).alpha(0.125),
        version_control_conflict_marker_theirs: color(RED).alpha(0.125),
    }
}

pub fn status_colors() -> StatusColors {
    StatusColors {
        conflict: color(RED),
        created: color(GREEN),
        created_background: color(GREEN).alpha(0.25),
        deleted: color(RED),
        deleted_background: color(RED).alpha(0.25),
        error: color(RED),
        error_background: color(RED).alpha(0.2),
        error_border: color(RED),
        hidden: color(TEXT_DIM),
        hint: color(TEXT_DIM),
        hint_background: color(POPUP),
        hint_border: color(BORDER),
        ignored: color(IGNORED),
        ignored_background: color(POPUP),
        ignored_border: color(BORDER),
        info: color(TEXT_DIM),
        info_background: color(POPUP),
        info_border: color(BORDER),
        modified: color(YELLOW),
        renamed: color(ORANGE),
        success: color(GREEN),
        success_background: color(POPUP),
        success_border: color(BORDER),
        unreachable: color(TEXT_MUTED),
        warning: color(YELLOW),
        warning_background: color(YELLOW).alpha(0.2),
        warning_border: color(YELLOW),
    }
}

/// The local cursor and selection.
pub fn cursor_colors() -> CursorColors {
    CursorColors { cursor: color(TEXT), selection: color(SELECTION).alpha(0.5) }
}

pub fn accent_colors() -> AccentColors {
    AccentColors([BLUE, CYAN, RED, ORANGE, YELLOW, GREEN, PURPLE].map(color))
}

pub fn system_colors() -> SystemColors {
    SystemColors {
        transparent: transparent_black(),
        mac_os_traffic_light_red: color(0xec6b5f),
        mac_os_traffic_light_yellow: color(0xf4bf4e),
        mac_os_traffic_light_green: color(0x62c454),
    }
}

pub fn syntax_theme() -> SyntaxTheme {
    let italic = |hex| HighlightStyle { font_style: Some(FontStyle::Italic), ..fg(hex) };
    let bold = |hex| HighlightStyle { font_weight: Some(FontWeight::BOLD), ..fg(hex) };
    SyntaxTheme::new(
        [
            ("attribute", fg(ORANGE)),
            ("boolean", fg(ORANGE)),
            ("comment", italic(COMMENT)),
            ("comment.doc", italic(COMMENT)),
            ("constant", fg(ORANGE)),
            ("constructor", fg(BLUE)),
            ("definition.function", fg(BLUE)),
            ("embedded", fg(TEXT)),
            ("emphasis", italic(BLUE)),
            ("emphasis.strong", bold(ORANGE)),
            ("enum", fg(YELLOW)),
            ("function", fg(BLUE)),
            ("function.method", fg(BLUE)),
            ("hint", fg(CODE_HINT)),
            ("keyword", fg(PURPLE)),
            ("label", fg(BLUE)),
            ("lifetime", fg(PURPLE)),
            ("link_text", HighlightStyle { font_style: Some(FontStyle::Normal), ..fg(BLUE) }),
            ("link_uri", fg(CYAN)),
            ("number", fg(ORANGE)),
            ("operator", fg(CYAN)),
            ("predictive", fg(CODE_HINT)),
            ("preproc", fg(PURPLE)),
            ("primary", fg(TEXT)),
            ("property", fg(RED)),
            ("punctuation", fg(TEXT)),
            ("punctuation.bracket", fg(TEXT)),
            ("punctuation.delimiter", fg(TEXT)),
            ("punctuation.list_marker", fg(RED)),
            ("punctuation.special", fg(TEXT)),
            ("string", fg(GREEN)),
            ("string.escape", fg(CYAN)),
            ("string.regex", fg(CYAN)),
            ("string.special", fg(ORANGE)),
            ("tag", fg(RED)),
            ("text.literal", fg(GREEN)),
            ("title", bold(RED)),
            ("type", fg(YELLOW)),
            ("type.builtin", fg(YELLOW)),
            ("type.super", fg(YELLOW)),
            ("variable", fg(TEXT)),
            ("variable.parameter", fg(TEXT)),
            ("variable.special", fg(RED)),
            ("variant", fg(ORANGE)),
        ]
        .map(|(name, style): (&str, HighlightStyle)| (name.to_owned(), style)),
    )
}

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn fg(hex: u32) -> HighlightStyle {
    HighlightStyle { color: Some(color(hex)), ..HighlightStyle::default() }
}

/// Scales each RGB channel by `100 + percent` percent, clamped: negative darkens, positive lightens.
const fn shade(hex: u32, percent: i32) -> u32 {
    let scale = (100 + percent) as u32;
    let mut out = 0;
    let mut shift = 0;
    while shift < 24 {
        let channel = ((hex >> shift) & 0xff) * scale / 100;
        out |= (if channel > 0xff { 0xff } else { channel }) << shift;
        shift += 8;
    }
    out
}

/// `top` at `percent` opacity flattened onto `bottom`, for surfaces that must stay opaque.
const fn mix(top: u32, bottom: u32, percent: u32) -> u32 {
    let mut out = 0;
    let mut shift = 0;
    while shift < 24 {
        let channel = (((top >> shift) & 0xff) * percent + ((bottom >> shift) & 0xff) * (100 - percent) + 50) / 100;
        out |= channel << shift;
        shift += 8;
    }
    out
}

#[cfg(test)]
mod tests {
    use gpui::FontWeight;
    use gpui::Rgba;

    use super::*;

    #[test]
    fn color_parses_rgb_hex() {
        for hex in [0x88c0d0, 0x3b4252, 0x000000] {
            let rgba = Rgba::from(color(hex));
            let channels = [rgba.r, rgba.g, rgba.b].map(|channel| (channel * 255.).round() as u32);
            assert_eq!((channels[0] << 16) | (channels[1] << 8) | channels[2], hex);
            assert_eq!(rgba.a, 1.);
        }
    }

    #[test]
    fn fg_sets_only_color() {
        assert_eq!(fg(RED), HighlightStyle { color: Some(color(RED)), ..HighlightStyle::default() });
    }

    #[test]
    fn shade_scales_and_clamps_channels() {
        assert_eq!(shade(DARK0, 0), DARK0);
        assert_eq!(shade(DARK0, -30), 0x1c1e24);
        assert_eq!(shade(RED, 20), 0xff818c);
        assert_eq!(shade(YELLOW, 20), 0xffe693);
        assert_eq!(shade(LIGHT0, -100), 0x000000);
    }

    #[test]
    fn mix_flattens_top_onto_bottom() {
        assert_eq!(mix(DARK1, DARK0, 100), DARK1);
        assert_eq!(mix(DARK1, DARK0, 0), DARK0);
        assert_eq!(mix(BORDER, CHROME, 50), 0x2d313b);
        assert_eq!(mix(BORDER, CHROME, 60), 0x30353f);
    }

    #[test]
    fn roles_derive_from_palette() {
        assert_eq!(CHROME, 0x1c1e24);
        assert_eq!(CARD, 0x121418);
        assert_eq!(POPUP, 0x2b2f38);
        assert_eq!(SELECTION, 0x8a94a8);
        assert_eq!(TEXT_MUTED, 0x888e98);
        assert_eq!(TEXT_DIM, 0x777c85);
        assert_eq!(CODE_HINT, 0x6e7686);
        assert_eq!(IGNORED, 0x897349);
    }

    #[test]
    fn theme_is_one_dark_with_floating_chrome() {
        let theme = theme();
        assert_eq!(theme.name.as_ref(), "One Dark");
        assert_eq!(theme.appearance, Appearance::Dark);

        let colors = theme.colors();
        assert_eq!(colors.background, colors.panel_background);
        assert_eq!(colors.title_bar_background, colors.panel_background);
        assert_eq!(colors.status_bar_background, colors.panel_background);
        assert_eq!(colors.tab_bar_background, colors.panel_background);
        assert_eq!(colors.tab_active_background, colors.editor_background);
        assert_eq!(colors.terminal_ansi_background, colors.terminal_background);
        assert!(colors.editor_background.l < colors.panel_background.l);
        assert!(colors.panel_overlay_hover.is_opaque());
        assert_eq!(colors.element_selection_background, theme.cursor_colors().selection);
    }

    #[test]
    fn terminal_dim_and_bright_bracket_normal() {
        let colors = theme_colors();
        for (dim, normal, bright) in [
            (colors.terminal_ansi_dim_red, colors.terminal_ansi_red, colors.terminal_ansi_bright_red),
            (colors.terminal_ansi_dim_green, colors.terminal_ansi_green, colors.terminal_ansi_bright_green),
            (colors.terminal_ansi_dim_blue, colors.terminal_ansi_blue, colors.terminal_ansi_bright_blue),
        ] {
            assert!(dim.l < normal.l && normal.l < bright.l);
        }
    }

    #[test]
    fn status_colors_use_popup_surfaces() {
        let status = status_colors();
        assert_eq!(status.ignored_background, status.hint_background);
        assert_eq!(status.ignored_border, status.hint_border);
        assert_eq!(status.error_background, color(RED).alpha(0.2));
    }

    #[test]
    fn syntax_theme_resolves_capture_names() {
        let theme = theme();
        let syntax = theme.syntax();
        assert_eq!(syntax.style_for_name("keyword").and_then(|style| style.color), Some(color(PURPLE)));
        assert_eq!(syntax.style_for_name("title").and_then(|style| style.font_weight), Some(FontWeight::BOLD));
        assert!(syntax.highlight_id("string.escape").is_some());
        assert_eq!(syntax.style_for_name("lifetime").and_then(|style| style.color), Some(color(PURPLE)));
        assert_eq!(syntax.style_for_name("variant").and_then(|style| style.color), Some(color(ORANGE)));
        assert!(syntax.style_for_name("missing").is_none());
    }

    #[test]
    fn theme_is_one_static_instance() {
        assert!(std::ptr::eq(theme(), theme()));
        assert_eq!(theme().syntax(), &syntax_theme());
    }

    #[test]
    fn accent_colors_cycle() {
        let accents = theme().accents();
        assert_eq!(accents.color_for_index(0), color(BLUE));
        assert_eq!(accents.color_for_index(accents.0.len() as u32), accents.color_for_index(0));
    }
}
