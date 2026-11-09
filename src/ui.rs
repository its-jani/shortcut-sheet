//! Design tokens and the pure parts of the view layer.
//!
//! Every colour literal in this crate lives in this file, mirroring
//! `replica/design/tokens.json`. The mirror is not a promise in a comment: `tests::
//! the_palette_mirrors_the_token_file` reads the token file back and fails the build when a
//! value drifts, because a spec nobody checks is a rumour.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};

pub mod overlay;

/// Hex to Color32, in const context. Written by hand because `Color32::from_hex` is not
/// const, and we want the palettes to be `const` items rather than lazily built.
const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb(((hex >> 16) & 0xFF) as u8, ((hex >> 8) & 0xFF) as u8, (hex & 0xFF) as u8)
}

/// The same for an `#RRGGBBAA` token. tokens.json writes its tints as alpha over whatever is
/// behind them, so the palette carries alpha too - keeping solid look-alikes would mean
/// every tint was secretly painted for one background only.
const fn rgba(hex: u32) -> Color32 {
    Color32::from_rgba_unmultiplied_const(
        ((hex >> 24) & 0xFF) as u8,
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub surface: Color32,
    /// The hub and drill card role (S02, S05). Nothing paints it yet - the search field used to
    /// borrow this role, and now has its own (`field`), which is what the measurement says the
    /// field actually is. Kept so the palette mirrors tokens.json one-for-one and nobody
    /// re-derives it when the hub lands.
    #[allow(dead_code)]
    pub card: Color32,
    /// The input box. Measured darker than the page it sits on (#1F1F1F on #222222) - the
    /// target's search field is a recessed box with a rule under it, not a raised white one,
    /// which is the difference egui's `extreme_bg_color` gets wrong by default.
    pub field: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    /// The settings page's disabled role (S07). egui 0.37 has no disabled-text colour of its
    /// own - `Visuals` only carries `weak_text_color` - so this waits for the screen that
    /// dims a row rather than being faked into the widget visuals.
    #[allow(dead_code)]
    pub text_disabled: Color32,
    pub accent: Color32,
    pub accent_muted: Color32,
    pub on_accent: Color32,
    pub keycap_fill: Color32,
    pub keycap_border: Color32,
    pub keycap_text: Color32,
    /// The drill's answered states (S05). Measured: a pressed key turns the cap white with a
    /// grey edge and a green check badge over its corner; a wrong key tints it red.
    #[allow(dead_code)]
    pub keycap_fill_correct: Color32,
    #[allow(dead_code)]
    pub keycap_border_correct: Color32,
    #[allow(dead_code)]
    pub keycap_text_correct: Color32,
    /// The empty drill slot's dashed outline.
    #[allow(dead_code)]
    pub slot_dashed: Color32,
    pub row_hover: Color32,
    pub row_selected: Color32,
    #[allow(dead_code)]
    pub success: Color32,
    #[allow(dead_code)]
    pub danger: Color32,
}

// tokens.json -> color.* (light ramp). Contrast verified AA in both themes.
pub const LIGHT: Palette = Palette {
    surface: rgb(0xFF_FF_FF),
    card: rgb(0xFF_FF_FF),
    field: rgb(0xFF_FF_FF),
    border: rgb(0xE3_E3_E3),
    border_strong: rgb(0xCF_CF_CF),
    text_primary: rgb(0x11_11_11),
    text_secondary: rgb(0x5C_5C_5C),
    text_tertiary: rgb(0x75_75_75),
    text_disabled: rgb(0x8A_8A_8A),
    accent: rgb(0x3C_6E_9F),
    accent_muted: rgba(0x3C_6E_9F_22),
    on_accent: rgb(0xFF_FF_FF),
    keycap_fill: rgb(0xE2_E2_E2),
    keycap_border: rgb(0xBF_BF_BF),
    keycap_text: rgb(0x4A_4A_4A),
    keycap_fill_correct: rgb(0xFF_FF_FF),
    keycap_border_correct: rgb(0x78_78_78),
    keycap_text_correct: rgb(0x11_11_11),
    slot_dashed: rgb(0x8A_8A_8A),
    row_hover: rgba(0x00_00_00_0A),
    row_selected: rgba(0x3C_6E_9F_1F),
    success: rgb(0x2E_7D_46),
    danger: rgb(0xB3_40_3A),
};

pub const DARK: Palette = Palette {
    surface: rgb(0x1F_1F_1F),
    card: rgb(0x2A_2A_2A),
    field: rgb(0x1F_1F_1F),
    border: rgb(0x38_38_38),
    border_strong: rgb(0x46_46_46),
    text_primary: rgb(0xFF_FF_FF),
    text_secondary: rgb(0xCB_CB_CB),
    text_tertiary: rgb(0xA8_A8_A8),
    text_disabled: rgb(0x80_80_80),
    accent: rgb(0x7F_AE_CE),
    accent_muted: rgba(0x7F_AE_CE_2E),
    on_accent: rgb(0x12_12_12),
    keycap_fill: rgb(0x23_23_23),
    keycap_border: rgb(0x46_46_46),
    keycap_text: rgb(0xE8_E8_E8),
    keycap_fill_correct: rgb(0xFF_FF_FF),
    keycap_border_correct: rgb(0x78_78_78),
    keycap_text_correct: rgb(0x11_11_11),
    slot_dashed: rgb(0x70_70_70),
    row_hover: rgba(0xFF_FF_FF_0F),
    row_selected: rgba(0x7F_AE_CE_26),
    success: rgb(0x72_DA_61),
    danger: rgb(0xF5_8C_A4),
};

/// egui's widget visuals - the text-edit field, the selection highlight, the scrollbars - are
/// a second colour system, separate from the palettes above, and they arrive with egui's own
/// defaults. Left unsynced, the light theme drew a near-black search box on a white panel.
///
/// Starts from the stock theme that matches the mode rather than building a `Visuals` literal,
/// so every field this file has no token for keeps working, and repaints only the roles
/// tokens.json names.
pub fn visuals(p: &Palette, dark: bool) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.surface;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    // `extreme_bg_color` is the fill `TextEdit` gives its frame - the one real input field in
    // the product. It is the `field` role, measured darker than the page behind it, not the
    // card: the target's search box is recessed, and egui's own default is near-black in both
    // themes.
    v.extreme_bg_color = p.field;
    v.faint_bg_color = p.field;
    v.hyperlink_color = p.accent;
    v.weak_text_color = Some(p.text_tertiary);
    v.selection.bg_fill = p.accent_muted;
    v.selection.stroke = Stroke::new(1.0, p.text_primary);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.fg_stroke.color = p.text_primary;
    w.noninteractive.bg_stroke.color = p.border;
    w.inactive.bg_fill = p.field;
    w.inactive.weak_bg_fill = p.field;
    w.inactive.fg_stroke.color = p.text_primary;
    w.inactive.bg_stroke.color = p.border_strong;
    w.hovered.bg_fill = p.row_selected;
    w.hovered.weak_bg_fill = p.row_selected;
    w.hovered.fg_stroke.color = p.text_primary;
    w.hovered.bg_stroke.color = p.accent;
    w.active.bg_fill = p.accent;
    w.active.weak_bg_fill = p.accent;
    w.active.fg_stroke.color = p.on_accent;
    w.active.bg_stroke.color = p.accent;
    v
}

/// Sizes from tokens.json -> size.*
///
/// Every number here is a measurement off `demo.mp4` or the light overlay composite, and
/// `the_palette_mirrors_the_token_file` checks them against the token file so the two cannot
/// quietly disagree.
pub mod size {
    pub const OVERLAY_WIDTH: f32 = 360.0;
    pub const OVERLAY_MAX_HEIGHT: f32 = 399.0;
    pub const ROW_HEIGHT: f32 = 32.0;
    pub const KEYCAP_HEIGHT: f32 = 23.0;
    pub const KEYCAP_MIN_WIDTH: f32 = 24.0;
    /// The chip grows around its label: 24 for one character, 35 for the word `Ctrl`. That is
    /// a ~6.3 advance per glyph plus 10 of horizontal padding, which is what a 12 px mono is.
    pub const KEYCAP_CHAR_STEP: f32 = 6.3;
    pub const KEYCAP_PAD_X: f32 = 10.0;
    pub const KEYCAP_GAP: f32 = 4.0;
    pub const TITLE_BAR_HEIGHT: f32 = 36.0;
    /// The field box plus the rule under it.
    pub const SEARCH_HEIGHT: f32 = 38.0;
    pub const COLLECTION_HEADER_HEIGHT: f32 = 25.0;
    pub const PADDING: f32 = 12.0;
    pub const SECTION_GAP: f32 = 10.0;
    /// tokens.json -> stroke.focus (2) and stroke.hairline (1): the search rule is 2 px in the
    /// accent when the field has focus and 1 px of border-strong when it does not.
    pub const STROKE_FOCUS: f32 = 2.0;
    pub const STROKE_HAIRLINE: f32 = 1.0;
    /// The drill's caps (S05): 64 tall, 62 for a character, 105 for a word, 18 apart.
    #[allow(dead_code)]
    pub const DRILL_KEYCAP_HEIGHT: f32 = 64.0;
    #[allow(dead_code)]
    pub const DRILL_KEYCAP_GAP: f32 = 18.0;
}

pub mod radius {
    use egui::CornerRadius;
    /// tokens.json -> radius.window (8)
    pub const WINDOW: CornerRadius = CornerRadius::same(8);
    /// tokens.json -> radius.keycap (4)
    pub const KEYCAP: CornerRadius = CornerRadius::same(4);
    /// tokens.json -> radius.card (4); a row highlight is the same corner as the card it is on.
    pub const ROW: CornerRadius = CornerRadius::same(4);
    /// tokens.json -> radius.field (4)
    pub const FIELD: CornerRadius = CornerRadius::same(4);
    /// tokens.json -> radius.drill_keycap (8)
    #[allow(dead_code)]
    pub const DRILL_KEYCAP: CornerRadius = CornerRadius::same(8);
}

/// Type roles from tokens.json -> type.roles. The sizes are cap heights off frames divided by
/// 0.696 (Segoe UI's cap ratio), not style names copied from a spec sheet.
pub mod font {
    use egui::FontId;
    /// type.roles.command_name -> md
    pub fn command() -> FontId {
        FontId::proportional(14.0)
    }
    /// type.roles.collection_header -> md (measured 10 px of cap in the overlay, which is 14,
    /// not the 12 the hub's smaller labels use)
    pub fn header() -> FontId {
        FontId::proportional(14.0)
    }
    /// type.roles.count / the no-match hint -> sm
    pub fn caption() -> FontId {
        FontId::proportional(12.0)
    }
    /// The chip label: 8 px of digit height measured inside a 23 tall cap is ~12 px of mono.
    pub fn keycap() -> FontId {
        FontId::monospace(12.0)
    }
    /// type.roles.overlay_title -> md
    pub fn title() -> FontId {
        FontId::proportional(14.0)
    }
    /// type.roles.page_title -> 2xl (the hub and settings headers, for when S02 is built)
    #[allow(dead_code)]
    pub fn page_title() -> FontId {
        FontId::proportional(28.0)
    }
}

/// Hover and selection are tints, not fills: the token carries the alpha, so this is the
/// identity today. It stays as the one place that says so, and as the hook for a hover that
/// has to composite differently once the panel can be translucent over arbitrary content.
pub fn hover_fill(p: &Palette) -> Color32 {
    p.row_hover
}

/// How wide a chip for this label has to be. Measured, not guessed: 24 device-logical px for
/// one character and 35 for the word `Ctrl`, which is a ~6.3 advance plus 10 of padding.
pub fn keycap_width(label: &str) -> f32 {
    (label.chars().count() as f32 * size::KEYCAP_CHAR_STEP + size::KEYCAP_PAD_X)
        .max(size::KEYCAP_MIN_WIDTH)
}

/// The drill's four cap states (S05). Kept as data rather than as four call sites, because the
/// one that is easy to get wrong is `Idle`: the target draws an empty dashed slot, not a grey
/// chip, and a filled slot reads as an answer already given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CapState {
    Idle,
    Held,
    Correct,
    Wrong,
}

/// Fill, border and text colour for one drill cap in one state, from the palette.
#[allow(dead_code)]
pub fn drill_cap_colors(p: &Palette, state: CapState) -> (Color32, Color32, Color32) {
    match state {
        // Measured: the answered cap turns white with a grey edge and dark text, in both themes.
        CapState::Correct => (p.keycap_fill_correct, p.keycap_border_correct, p.keycap_text_correct),
        CapState::Wrong => (p.danger, p.danger, p.surface),
        CapState::Held => (p.accent_muted, p.accent, p.keycap_text),
        CapState::Idle => (p.field, p.slot_dashed, p.text_tertiary),
    }
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

/// An alias is a courtesy, not a second title: a row whose own name matches is the better
/// answer than a row only reachable through a name the user cannot see. The discount is
/// smaller than the gap between match bands (100_000), so an alias prefix still beats a
/// mid-word hit on the real name, and an alias subsequence still loses to one.
const ALIAS_DECAY: i64 = 50_000;

/// The best score a query gets across every name one row goes by.
///
/// `names[0]` is the command as it is drawn; everything after it is an alias. Without this,
/// typing "inspect element" finds nothing while the row that *is* inspect element sits there
/// under its menu's name.
pub fn best_score(names: &[&str], needle: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0);
    }
    names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            let score = search_score(name, needle)?;
            Some(if i == 0 { score } else { score - ALIAS_DECAY })
        })
        .max()
}

/// One row as the search sees it: the names it answers to, and where it lives in the profile.
#[derive(Debug, Clone)]
pub struct SearchRow<'a> {
    /// Command name first, then its aliases. `Shortcut::names` produces this shape.
    pub names: Vec<&'a str>,
    pub collection: usize,
    pub shortcut: usize,
}

/// Rank rows by relevance to `query`, keeping the dataset order for ties. Empty query returns
/// everything, unsorted.
pub fn rank(rows: &[SearchRow<'_>], query: &str) -> Vec<(usize, usize)> {
    let mut scored: Vec<(i64, usize, usize)> = rows
        .iter()
        .filter_map(|r| {
            best_score(&r.names, query).map(|score| (score, r.collection, r.shortcut))
        })
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

    /// The design spec, read at compile time by `the_palette_mirrors_the_token_file`.
    const TOKENS_JSON: &str = include_str!("../replica/design/tokens.json");

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

    /// The rows the tests talk about, in the shape `Shortcut::names` produces: name first,
    /// aliases after it.
    fn row(names: &[&'static str], collection: usize, shortcut: usize) -> SearchRow<'static> {
        SearchRow { names: names.to_vec(), collection, shortcut }
    }

    #[test]
    fn rank_preserves_order_for_equal_scores() {
        // Same length, same prefix - the scorer gives both the same number, so the only
        // thing left to order them by is the order they appear in the dataset.
        let items = vec![row(&["Copy"], 0, 0), row(&["Crop"], 0, 1)];
        let r = rank(&items, "c");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], (0, 0), "ties keep dataset order");
    }

    #[test]
    fn a_shorter_name_outranks_a_longer_one_on_the_same_prefix() {
        // Deliberate: typing "c" in Chrome's list should put "Copy" above "Collapse all
        // folders", because the tighter match is the more likely one.
        let items = vec![
            row(&["Collapse all folders"], 0, 0),
            row(&["Copy"], 0, 1),
        ];
        assert_eq!(rank(&items, "c"), vec![(0, 1), (0, 0)]);
    }

    #[test]
    fn rank_drops_non_matches() {
        let items = vec![row(&["Copy"], 0, 0), row(&["Paste"], 0, 1)];
        assert_eq!(rank(&items, "cop"), vec![(0, 0)]);
    }

    #[test]
    fn an_empty_query_keeps_every_row_in_dataset_order() {
        let items = vec![row(&["Copy"], 0, 0), row(&["Paste"], 0, 1)];
        assert_eq!(rank(&items, ""), vec![(0, 0), (0, 1)]);
    }

    /// The alias half of the search. These are the real dataset's rows, because the point of
    /// the feature is that typing the other name of a command has to find it.
    #[test]
    fn a_row_can_be_found_by_a_name_it_does_not_display() {
        let devtools = row(&["Developer tools", "inspect element"], 1, 2);
        let copy = row(&["Copy"], 0, 0);
        let items = vec![copy.clone(), devtools.clone()];

        // Nothing in either visible name contains "inspect", so without alias matching the
        // answer to this query is an empty list.
        assert_eq!(rank(&items, "inspect"), vec![(1, 2)]);
        assert_eq!(rank(&items, "element"), vec![(1, 2)]);
        assert!(rank(&items, "zoom").is_empty(), "a query that matches no name matches nothing");
    }

    #[test]
    fn every_alias_of_a_row_is_searchable_not_just_the_first() {
        let items = vec![row(
            &["Quick switcher", "jump to a channel", "server picker"],
            2,
            7,
        )];
        for query in ["quick", "jump", "picker", "server"] {
            assert_eq!(rank(&items, query), vec![(2, 7)], "{query} should find the row");
        }
    }

    #[test]
    fn a_match_on_the_visible_name_outranks_the_same_match_on_an_alias() {
        // "Focus address bar" is drawn, "omnibox" is not. Typing a prefix both rows can reach
        // has to put the one the user can read at the top.
        let by_alias = row(&["Focus address bar", "omnibox"], 0, 0);
        let by_name = row(&["Omni box suggestions"], 0, 1);
        let items = vec![by_alias.clone(), by_name.clone()];
        assert_eq!(rank(&items, "omn").first(), Some(&(0, 1)), "the visible name wins the tie");
        // And a weak alias match cannot outscore any real match on the label.
        assert_eq!(rank(&items, "address"), vec![(0, 0)]);
    }

    #[test]
    fn alias_discount_never_flips_a_match_into_a_non_match() {
        // A discount that pushed a score below the pass line would make a row unfindable by
        // the very name it was added for.
        assert!(best_score(&["Quick switcher", "jump to a channel"], "jump")
            .is_some());
        assert!(best_score(&["Quick switcher", "jump to a channel"], "quick").is_some());
    }

    #[test]
    fn row_split_leaves_keycaps_the_room_they_need() {
        let row = Rect::from_min_max(Pos2::ZERO, Pos2::new(size::OVERLAY_WIDTH, size::ROW_HEIGHT));
        let (name, keys) = split_row(row, 90.0);
        assert_eq!(keys.max.x, size::OVERLAY_WIDTH);
        assert!(name.max.x <= keys.min.x, "name must not run under the keycaps");
        assert!(name.width() > 100.0, "name still has room");
    }

    /// The two chip widths measured off the composite: a single character is the minimum, and
    /// the word `Ctrl` is 35. Getting this wrong is what makes the name column truncate early
    /// or run under the caps.
    #[test]
    fn keycap_width_matches_the_measured_chips() {
        assert_eq!(keycap_width("L"), 24.0, "one character is the minimum");
        assert!(
            (keycap_width("Ctrl") - 35.0).abs() < 1.5,
            "the word Ctrl measured 35, got {}",
            keycap_width("Ctrl")
        );
        assert!(keycap_width("Shift") > keycap_width("Ctrl"), "a longer word is a wider chip");
    }

    #[test]
    fn drill_caps_answer_in_order() {
        for p in [&LIGHT, &DARK] {
            let (idle_fill, idle_border, idle_text) = drill_cap_colors(p, CapState::Idle);
            assert_eq!(idle_border, p.slot_dashed, "the empty slot is the dashed role");

            let (fill, border, text) = drill_cap_colors(p, CapState::Correct);
            assert_eq!((fill, border), (p.keycap_fill_correct, p.keycap_border_correct));
            assert_eq!(text, p.keycap_text_correct);
            // The two states must never be paintable as the same triple. In the light theme the
            // recessed field and the answered cap are both white, so it is the border and the
            // label colour that carry the difference - which is exactly how the target does it.
            assert_ne!(
                (idle_fill, idle_border, idle_text),
                (fill, border, text),
                "an empty slot must not look like an answer"
            );
            assert_ne!(idle_text, text, "the prompt grey and the answer ink differ");

            assert_eq!(drill_cap_colors(p, CapState::Wrong).0, p.danger);
        }
    }

    #[test]
    fn key_labels_come_from_the_key_model() {
        use crate::data::key::Key;
        assert_eq!(Key::Ctrl.label(), "Ctrl");
        assert_eq!(Key::Grave.label(), "`");
        // The keys a custom trigger usually gets pointed at, whose labels have to stay narrow
        // enough to fit a chip.
        assert_eq!(Key::CapsLock.label(), "Caps");
        assert_eq!(Key::Apps.label(), "Menu");
    }

    /// The hover row is a *tint*: a low-alpha wash over whatever is behind it, per tokens.json
    /// `state.row_hover`. It is pinned here because a screen grab of a GL-composited transparent
    /// window came back twice with strips of the page behind in place of the panel, which looked
    /// exactly like an opaque white row and sent everyone hunting a paint bug that was in the
    /// probe. One test settles that argument without a screenshot.
    ///
    /// The bytes are premultiplied, because that is what `Color32` stores: a white 6% tint is
    /// `(15,15,15,15)`, not `(255,255,255,15)`. The old palette kept the second form by baking
    /// alpha onto a solid colour, which is not a valid premultiplied value - egui read it as a
    /// much brighter wash than the token asked for.
    #[test]
    fn hover_fill_stays_translucent_in_both_themes() {
        let dark = hover_fill(&DARK);
        let light = hover_fill(&LIGHT);
        assert!(dark.a() > 0 && dark.a() < 40, "dark hover must tint, not fill: {dark:?}");
        assert!(light.a() > 0 && light.a() < 40, "light hover must tint, not fill: {light:?}");
        // Premultiplied white: every channel equals the alpha. Premultiplied black: every
        // channel is zero. Either way the row is a wash over the panel, never a fill.
        assert_eq!(
            [dark.r(), dark.g(), dark.b()],
            [dark.a(), dark.a(), dark.a()],
            "dark hover is white at its own alpha: {dark:?}"
        );
        assert_eq!(
            [light.r(), light.g(), light.b()],
            [0, 0, 0],
            "light hover is black at its own alpha: {light:?}"
        );
    }

    /// The header of this file promises that every colour literal mirrors tokens.json. A
    /// promise in a comment is how the last palette drift got written, so this reads the token
    /// file back and compares it field by field - colours, sizes and radii. Change one side
    /// and the other has to move in the same commit.
    #[test]
    fn the_palette_mirrors_the_token_file() {
        let doc: serde_json::Value =
            serde_json::from_str(TOKENS_JSON).expect("replica/design/tokens.json must parse");
        let hex = |path: &[&str]| -> Color32 {
            let mut node = &doc;
            for key in path {
                node = &node[*key];
            }
            let s = node
                .as_str()
                .unwrap_or_else(|| panic!("{path:?} is not a hex string in tokens.json"));
            let h = s.trim_start_matches('#');
            let byte = |i: usize| u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).expect("hex byte");
            if h.len() == 6 {
                Color32::from_rgb(byte(0), byte(1), byte(2))
            } else {
                Color32::from_rgba_unmultiplied(byte(0), byte(1), byte(2), byte(3))
            }
        };
        let num = |path: &[&str]| -> f32 {
            let mut node = &doc;
            for key in path {
                node = &node[*key];
            }
            node.as_f64().unwrap_or_else(|| panic!("{path:?} is not a number in tokens.json"))
                as f32
        };

        for (p, mode) in [(&LIGHT, "light"), (&DARK, "dark")] {
            let role = |group: &str, key: &str| -> Color32 {
                hex(&["color", group, key, mode])
            };
            assert_eq!(p.surface, role("overlay", "surface"), "{mode} surface");
            assert_eq!(p.card, role("overlay", "card"), "{mode} card");
            assert_eq!(p.field, role("page", "field"), "{mode} field");
            assert_eq!(p.border, role("overlay", "border"), "{mode} border");
            assert_eq!(p.border_strong, role("overlay", "border-strong"), "{mode} border-strong");
            assert_eq!(p.text_primary, role("text", "primary"), "{mode} text.primary");
            assert_eq!(p.text_secondary, role("text", "secondary"), "{mode} text.secondary");
            assert_eq!(p.text_tertiary, role("text", "tertiary"), "{mode} text.tertiary");
            assert_eq!(p.text_disabled, role("text", "disabled"), "{mode} text.disabled");
            assert_eq!(p.on_accent, role("text", "on-accent"), "{mode} text.on-accent");
            assert_eq!(p.accent, role("accent", "base"), "{mode} accent.base");
            assert_eq!(p.accent_muted, role("accent", "muted"), "{mode} accent.muted");
            assert_eq!(p.keycap_fill, role("keycap", "fill"), "{mode} keycap.fill");
            assert_eq!(p.keycap_border, role("keycap", "border"), "{mode} keycap.border");
            assert_eq!(p.keycap_text, role("keycap", "text"), "{mode} keycap.text");
            assert_eq!(
                p.keycap_fill_correct,
                role("keycap", "fill-correct"),
                "{mode} keycap.fill-correct"
            );
            assert_eq!(
                p.keycap_border_correct,
                role("keycap", "border-correct"),
                "{mode} keycap.border-correct"
            );
            assert_eq!(
                p.keycap_text_correct,
                role("keycap", "text-correct"),
                "{mode} keycap.text-correct"
            );
            assert_eq!(p.slot_dashed, role("control", "slot-dashed"), "{mode} control.slot-dashed");
            assert_eq!(p.row_hover, role("state", "row_hover"), "{mode} state.row_hover");
            assert_eq!(p.row_selected, role("state", "row_selected"), "{mode} state.row_selected");
            assert_eq!(p.success, role("state", "success"), "{mode} state.success");
            assert_eq!(p.danger, role("state", "danger"), "{mode} state.danger");
        }

        assert_eq!(size::OVERLAY_WIDTH, num(&["size", "overlay_width"]), "overlay_width");
        assert_eq!(size::OVERLAY_MAX_HEIGHT, num(&["size", "overlay_max_height"]), "max height");
        assert_eq!(size::ROW_HEIGHT, num(&["size", "row_height"]), "row_height");
        assert_eq!(size::KEYCAP_HEIGHT, num(&["size", "keycap_height"]), "keycap_height");
        assert_eq!(size::KEYCAP_MIN_WIDTH, num(&["size", "keycap_min_width"]), "min width");
        assert_eq!(size::KEYCAP_GAP, num(&["size", "keycap_gap"]), "keycap_gap");
        assert_eq!(size::TITLE_BAR_HEIGHT, num(&["size", "title_bar_height"]), "title bar");
        assert_eq!(size::SEARCH_HEIGHT, num(&["size", "search_height"]), "search height");
        assert_eq!(
            size::COLLECTION_HEADER_HEIGHT,
            num(&["size", "collection_header_height"]),
            "collection header"
        );
        assert_eq!(size::PADDING, num(&["space", "overlay_padding"]), "overlay_padding");
        assert_eq!(size::SECTION_GAP, num(&["space", "overlay_section_gap"]), "section gap");
        assert_eq!(
            size::DRILL_KEYCAP_HEIGHT,
            num(&["size", "drill_keycap_height"]),
            "drill cap height"
        );
        assert_eq!(
            size::DRILL_KEYCAP_GAP,
            num(&["size", "drill_keycap_gap"]),
            "drill cap gap"
        );

        assert_eq!(radius::WINDOW.nw, num(&["radius", "window"]) as u8);
        assert_eq!(radius::KEYCAP.nw, num(&["radius", "keycap"]) as u8);
        assert_eq!(radius::ROW.nw, num(&["radius", "card"]) as u8);
        assert_eq!(radius::FIELD.nw, num(&["radius", "field"]) as u8);
        assert_eq!(radius::DRILL_KEYCAP.nw, num(&["radius", "drill_keycap"]) as u8);

        // And the type roles: the font helpers must be the sizes the roles name.
        let role_size = |role: &str| -> f32 {
            let token =
                doc["type"]["roles"][role]["size"].as_str().expect("role size is a scale name");
            num(&["type", "scale", token])
        };
        assert_eq!(font::command().size, role_size("command_name"), "command_name");
        assert_eq!(font::header().size, role_size("collection_header"), "collection_header");
        assert_eq!(font::title().size, role_size("overlay_title"), "overlay_title");
        assert_eq!(font::caption().size, num(&["type", "scale", "sm"]), "caption is the sm step");
        assert_eq!(font::page_title().size, role_size("page_title"), "page_title");
    }

    /// The search field is the only widget on the overlay that egui paints itself, and egui
    /// fills it with `extreme_bg_color` - which is not part of our palette and defaults to
    /// near-black in both stock themes. This is the real bug the light-theme screenshot caught:
    /// a white panel with a black hole in it. Every role `visuals` claims to map must be
    /// checked, or the sync silently stops covering new fields.
    #[test]
    fn egui_visuals_are_painted_from_the_palette() {
        for (p, dark) in [(&LIGHT, false), (&DARK, true)] {
            let v = visuals(p, dark);
            assert_eq!(v.extreme_bg_color, p.field, "text-edit frame is the field role");
            assert_eq!(v.panel_fill, p.surface, "panel behind the overlay");
            assert_eq!(v.window_stroke.color, p.border, "hairline border");
            assert_eq!(v.hyperlink_color, p.accent, "links");
            assert_eq!(v.weak_text_color, Some(p.text_tertiary), "hint text");
            assert_eq!(v.selection.bg_fill, p.accent_muted, "selected text");
            assert_eq!(v.widgets.inactive.bg_stroke.color, p.border_strong, "field border");
            assert_eq!(v.widgets.inactive.bg_fill, p.field, "unfocused field fill");
            assert_eq!(v.widgets.active.fg_stroke.color, p.on_accent, "text on accent");
            // The mode flag has to agree with the palette, or egui picks contrast for us.
            assert_eq!(v.dark_mode, dark);
        }
        // And the two themes must not collapse onto the same colours - the light field is
        // white, the dark field is not.
        assert_ne!(visuals(&LIGHT, false).extreme_bg_color, visuals(&DARK, true).extreme_bg_color);
    }
}
