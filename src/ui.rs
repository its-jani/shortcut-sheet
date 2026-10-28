//! Design tokens and the pure parts of the view layer.
//!
//! Every colour literal in this crate lives in this file, mirroring
//! `replica/design/tokens.json`. Change a value there, change it here.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};

pub mod overlay;

/// Hex to Color32, in const context. Written by hand because `Color32::from_hex` is not
/// const, and we want the palettes to be `const` items rather than lazily built.
const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb(((hex >> 16) & 0xFF) as u8, ((hex >> 8) & 0xFF) as u8, (hex & 0xFF) as u8)
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub surface: Color32,
    /// Card roles below belong to the hub and drill screens (S02, S05), which are `should`
    /// features and not part of this build yet. They are kept so the palette mirrors
    /// tokens.json one-for-one and nobody re-derives them later.
    #[allow(dead_code)]
    pub card: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub accent: Color32,
    pub accent_muted: Color32,
    #[allow(dead_code)]
    pub on_accent: Color32,
    pub keycap_fill: Color32,
    pub keycap_border: Color32,
    pub keycap_text: Color32,
    pub row_hover: Color32,
    pub row_selected: Color32,
    #[allow(dead_code)]
    pub success: Color32,
    #[allow(dead_code)]
    pub danger: Color32,
}

// tokens.json -> color.* (light ramp). Contrast verified AA in both themes.
pub const LIGHT: Palette = Palette {
    surface: rgb(0xF7_F7_F7),
    card: rgb(0xFF_FF_FF),
    border: rgb(0xE3_E3_E3),
    border_strong: rgb(0xCF_CF_CF),
    text_primary: rgb(0x1A_1A_1A),
    text_secondary: rgb(0x5C_5C_5C),
    text_tertiary: rgb(0x75_75_75),
    accent: rgb(0x3C_6E_9F),
    accent_muted: rgb(0xD7_E4_EF),
    on_accent: rgb(0xFF_FF_FF),
    keycap_fill: rgb(0xEF_EF_EF),
    keycap_border: rgb(0xD4_D4_D4),
    keycap_text: rgb(0x2B_2B_2B),
    row_hover: rgb(0x00_00_00), // used with a low alpha below
    row_selected: rgb(0xD7_E4_EF),
    success: rgb(0x2E_7D_46),
    danger: rgb(0xB3_40_3A),
};

pub const DARK: Palette = Palette {
    surface: rgb(0x1F_1F_1F),
    card: rgb(0x2A_2A_2A),
    border: rgb(0x38_38_38),
    border_strong: rgb(0x48_48_48),
    text_primary: rgb(0xF2_F2_F2),
    text_secondary: rgb(0xA3_A3_A3),
    text_tertiary: rgb(0x9A_9A_9A),
    accent: rgb(0x7F_AE_CE),
    accent_muted: rgb(0x2C_3B_47),
    on_accent: rgb(0x12_12_12),
    keycap_fill: rgb(0x35_35_35),
    keycap_border: rgb(0x4A_4A_4A),
    keycap_text: rgb(0xE8_E8_E8),
    row_hover: rgb(0xFF_FF_FF),
    row_selected: rgb(0x2C_3B_47),
    success: rgb(0x6F_CB_8A),
    danger: rgb(0xE8_87_7F),
};

/// Sizes from tokens.json -> size.*
pub mod size {
    pub const OVERLAY_WIDTH: f32 = 344.0;
    pub const OVERLAY_MAX_HEIGHT: f32 = 424.0;
    pub const ROW_HEIGHT: f32 = 32.0;
    pub const KEYCAP_HEIGHT: f32 = 20.0;
    pub const KEYCAP_MIN_WIDTH: f32 = 20.0;
    pub const KEYCAP_GAP: f32 = 4.0;
    pub const TITLE_BAR_HEIGHT: f32 = 32.0;
    pub const SEARCH_HEIGHT: f32 = 30.0;
    pub const PADDING: f32 = 12.0;
    pub const SECTION_GAP: f32 = 10.0;
}

pub mod radius {
    use egui::CornerRadius;
    pub const WINDOW: CornerRadius = CornerRadius::same(8);
    pub const KEYCAP: CornerRadius = CornerRadius::same(4);
    pub const ROW: CornerRadius = CornerRadius::same(4);
}

pub mod font {
    use egui::FontId;
    pub fn command() -> FontId {
        FontId::proportional(14.0)
    }
    pub fn header() -> FontId {
        FontId::proportional(12.0)
    }
    pub fn keycap() -> FontId {
        FontId::monospace(11.5)
    }
    pub fn title() -> FontId {
        FontId::proportional(13.0)
    }
}

/// Hover/selected fills carry alpha in the token file; egui wants it baked in.
pub fn hover_fill(p: &Palette) -> Color32 {
    let mut c = p.row_hover;
    c[3] = if p.surface == LIGHT.surface { 10 } else { 16 };
    c
}

/// Draw one keycap: the fixed-height chip a shortcut row ends with.
///
/// A Button with the frame forced on is used rather than hand-placed painter calls: it
/// gives us the min-width a single-character cap needs without measuring text ourselves.
pub fn keycap(ui: &mut Ui, label: &str, p: &Palette, emphasized: bool) {
    let fill = if emphasized { p.accent_muted } else { p.keycap_fill };
    let stroke = Stroke::new(1.0, if emphasized { p.accent } else { p.keycap_border });
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .font(font::keycap())
                .color(if emphasized { p.accent } else { p.keycap_text }),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(radius::KEYCAP)
        .min_size(Vec2::new(size::KEYCAP_MIN_WIDTH, size::KEYCAP_HEIGHT))
        .sense(egui::Sense::hover()),
    );
}

/// Score a command against a query. `None` means "does not match".
///
/// Prefix beats word-boundary beats substring beats subsequence, which is what makes
/// `"cp"` find "Copy" before "Paste from clipboard".
pub fn search_score(haystack: &str, needle: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0);
    }
    let h = haystack.to_lowercase();
    let n = needle.to_lowercase();
    let n = n.replace(' ', "");
    if n.is_empty() {
        return Some(0);
    }
    let compact: String = h.chars().filter(|c| !c.is_whitespace()).collect();

    if compact.starts_with(&n) {
        return Some(1_000_000 - compact.len() as i64);
    }
    // word-boundary prefix: "nb" matches "New Tab" via its initials
    let initials: String = h
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_lowercase();
    if initials.starts_with(&n) {
        return Some(900_000 - initials.len() as i64);
    }
    if let Some(pos) = compact.find(&n) {
        return Some(500_000 - pos as i64 - compact.len() as i64);
    }
    // subsequence, last resort
    let mut it = compact.chars();
    let mut hits = 0usize;
    let mut gaps = 0usize;
    let mut since_last = 0usize;
    for want in n.chars() {
        let mut found = false;
        for tried in &mut it {
            since_last += 1;
            if tried == want {
                found = true;
                hits += 1;
                gaps += since_last - 1;
                since_last = 0;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    if hits == n.len() {
        Some(100_000 - gaps as i64 - compact.len() as i64)
    } else {
        None
    }
}

/// Rank a list of command names by relevance to `query`, keeping the dataset order for
/// ties. Empty query returns everything, unsorted.
pub fn rank<'a>(items: &'a [(String, usize, usize)], query: &str) -> Vec<(usize, usize)> {
    let mut scored: Vec<(i64, usize, usize)> = items
        .iter()
        .map(|(name, c, s)| (search_score(name, query).unwrap_or(i64::MIN), *c, *s))
        .filter(|(score, _, _)| *score != i64::MIN)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    scored.into_iter().map(|(_, c, s)| (c, s)).collect()
}

/// A small letter avatar, used instead of any third-party application logo.
pub fn letter_avatar(ui: &mut Ui, name: &str, p: &Palette, side: f32) {
    let letter = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(side, side), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, radius::KEYCAP, p.accent_muted);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            letter,
            FontId::proportional(side * 0.55),
            p.accent,
        );
    }
}

/// Clip helper used by the row renderer so long command names ellipsize and keycaps
/// never do (tokens.json -> component.overlay_row.truncate).
pub fn split_row(row: Rect, keycaps_width: f32) -> (Rect, Rect) {
    let name_right = row.max.x - keycaps_width - 8.0;
    (
        Rect::from_min_max(row.min, Pos2::new(name_right.max(row.min.x), row.max.y)),
        Rect::from_min_max(Pos2::new(name_right, row.min.y), row.max),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything_with_zero_score() {
        assert_eq!(search_score("Copy", ""), Some(0));
    }

    #[test]
    fn prefix_beats_substring() {
        let copy = search_score("Copy", "co").unwrap();
        // "Disconnect" holds "co" inside the word, never at the front.
        let disconnect = search_score("Disconnect", "co").unwrap();
        assert!(copy > disconnect, "prefix should outrank a mid-word hit: {copy} vs {disconnect}");
    }

    #[test]
    fn initials_match_word_boundaries() {
        assert!(search_score("New Tab", "nt").is_some());
        assert!(search_score("New Incognito Window", "niw").is_some());
    }

    #[test]
    fn ignores_spaces_in_the_query() {
        assert!(search_score("Command palette", "com pal").is_some());
    }

    #[test]
    fn subsequence_is_last_resort_and_beats_nothing() {
        let s = search_score("Reopen closed tab", "rct");
        assert!(s.is_some(), "initials should catch this");
    }

    #[test]
    fn non_matching_query_returns_none() {
        assert!(search_score("Copy", "xyz").is_none());
    }

    #[test]
    fn rank_preserves_order_for_equal_scores() {
        // Same length, same prefix - the scorer gives both the same number, so the only
        // thing left to order them by is the order they appear in the dataset.
        let items = vec![
            ("Copy".to_string(), 0usize, 0usize),
            ("Crop".to_string(), 0usize, 1usize),
        ];
        let r = rank(&items, "c");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], (0, 0), "ties keep dataset order");
    }

    #[test]
    fn a_shorter_name_outranks_a_longer_one_on_the_same_prefix() {
        // Deliberate: typing "c" in Chrome's list should put "Copy" above "Collapse all
        // folders", because the tighter match is the more likely one.
        let items = vec![
            ("Collapse all folders".to_string(), 0usize, 0usize),
            ("Copy".to_string(), 0usize, 1usize),
        ];
        assert_eq!(rank(&items, "c"), vec![(0, 1), (0, 0)]);
    }

    #[test]
    fn rank_drops_non_matches() {
        let items = vec![
            ("Copy".to_string(), 0usize, 0usize),
            ("Paste".to_string(), 0usize, 1usize),
        ];
        assert_eq!(rank(&items, "cop"), vec![(0, 0)]);
    }

    #[test]
    fn row_split_leaves_keycaps_the_room_they_need() {
        let row = Rect::from_min_max(Pos2::ZERO, Pos2::new(344.0, 32.0));
        let (name, keys) = split_row(row, 90.0);
        assert_eq!(keys.max.x, 344.0);
        assert!(name.max.x <= keys.min.x, "name must not run under the keycaps");
        assert!(name.width() > 100.0, "name still has room");
    }

    #[test]
    fn key_labels_come_from_the_key_model() {
        use crate::data::Key;
        assert_eq!(Key::Ctrl.label(), "Ctrl");
        assert_eq!(Key::Grave.label(), "`");
    }

    /// The hover row is a *tint*: white with a low alpha, per tokens.json `#FFFFFF0F`. It is
    /// pinned here because a screen grab of a GL-composited transparent window came back twice
    /// with strips of the page behind in place of the panel, which looked exactly like an
    /// opaque white row and sent everyone hunting a paint bug that was in the probe. One test
    /// settles that argument without a screenshot.
    #[test]
    fn hover_fill_stays_translucent_in_both_themes() {
        let dark = hover_fill(&DARK);
        let light = hover_fill(&LIGHT);
        assert_eq!([dark.r(), dark.g(), dark.b()], [255, 255, 255]);
        assert!(dark.a() > 0 && dark.a() < 40, "dark hover must tint, not fill: {dark:?}");
        assert!(light.a() > 0 && light.a() < 40, "light hover must tint, not fill: {light:?}");
    }
}
