//! S01 - the cheat-sheet overlay.

use egui::{Align, Color32, Layout, Pos2, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::data::{AppProfile, Catalog, Combo};
use crate::detect;
use crate::ui::{font, hover_fill, keycap, radius, rank, size, Palette};

/// What the overlay decided during this frame.
#[derive(Debug, PartialEq)]
pub enum Action {
    Nothing,
    /// Fire this chord into the target app, then close.
    Fire(Combo),
    /// Hand keyboard focus to the overlay so the search box can be typed in.
    WantFocus,
    /// Give focus back to the app the user came from and close.
    GiveUpFocus,
}

/// One resolved row, ready to draw.
struct Row {
    collection: usize,
    shortcut: usize,
    name: String,
    keys: String,
}

#[derive(Default)]
pub struct Overlay {
    pub app: Option<String>,
    query: String,
    selected: usize,
    rows: Vec<Row>,
    /// True while the overlay deliberately holds keyboard focus (search in use).
    pub focused: bool,
    /// Set by the key handler, resolved in `list` where the profile is in scope.
    pending_enter: bool,
}

impl Overlay {
    /// Load a profile into the view. Called on every show.
    pub fn open(&mut self, catalog: &Catalog, app_name: &str) {
        let Some(app) = catalog.app(app_name) else { return };
        self.app = Some(app.name.clone());
        self.query.clear();
        self.selected = 0;
        self.focused = false;
        self.pending_enter = false;
        self.rebuild(app);
    }

    pub fn close(&mut self) {
        self.app = None;
        self.query.clear();
        self.selected = 0;
        self.focused = false;
        self.rows.clear();
    }

    pub fn is_open(&self) -> bool {
        self.app.is_some()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The combo for the currently highlighted row, if any.
    pub fn selected_combo(&self) -> Option<Combo> {
        let row = self.rows.get(self.selected)?;
        Combo::parse(&row.keys).ok()
    }

    /// The highlighted row as (collection, command), for the progress record written when
    /// a shortcut is fired.
    pub fn selected_entry(&self, catalog: &Catalog) -> Option<(String, String)> {
        let app = catalog.app(self.app.as_deref()?)?;
        let row = self.rows.get(self.selected)?;
        let col = &app.collections[row.collection];
        Some((col.name.clone(), col.shortcuts[row.shortcut].command.clone()))
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.rows.is_empty() {
            return;
        }
        let n = self.rows.len() as i32;
        let cur = self.selected as i32;
        self.selected = (cur + delta).clamp(0, n - 1) as usize;
    }

    fn rebuild(&mut self, app: &AppProfile) {
        let items: Vec<(String, usize, usize)> = detect::visible_rows(app)
            .into_iter()
            .map(|(_, ci, si)| (app.collections[ci].shortcuts[si].command.clone(), ci, si))
            .collect();

        let order = rank(&items, &self.query);
        self.rows = order
            .into_iter()
            .map(|(ci, si)| {
                let s = &app.collections[ci].shortcuts[si];
                Row {
                    collection: ci,
                    shortcut: si,
                    name: s.command.clone(),
                    keys: s.keys.clone(),
                }
            })
            .collect();

        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
    }

    /// Draw the panel inside the already-sized overlay window.
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        catalog: &Catalog,
        p: &Palette,
        keys: &KeyFlags,
    ) -> Action {
        let Some(name) = self.app.clone() else { return Action::Nothing };
        let Some(app) = catalog.app(&name) else { return Action::Nothing };

        // Keys only reach us in focused mode; in browse mode the target app still owns them.
        if self.focused {
            if keys.escape {
                return Action::GiveUpFocus;
            }
            if keys.down {
                self.move_selection(1);
            }
            if keys.up {
                self.move_selection(-1);
            }
            if keys.enter {
                self.pending_enter = true;
            }
        }

        let before = self.rows.len();
        let before_query = self.query.clone();
        let mut action = Action::Nothing;

        let frame = egui::Frame::NONE
            .fill(p.surface)
            .corner_radius(radius::WINDOW)
            .inner_margin(egui::Margin::same(size::PADDING as i8))
            .stroke(Stroke::new(1.0, p.border));

        frame.show(ui, |ui| {
            ui.set_width(size::OVERLAY_WIDTH - size::PADDING * 2.0);
            self.title_bar(ui, p, app, &mut action);
            self.search_bar(ui, p, app, &mut action);
            ui.add_space(6.0);
            self.list(ui, p, app, &mut action);
        });

        // Re-filter only when the text actually changed, so we do not rebuild every frame.
        if self.query != before_query || (self.query.is_empty() && before != self.rows.len()) {
            self.rebuild(app);
            if self.query != before_query {
                self.selected = 0;
            }
        }

        action
    }

    fn title_bar(&mut self, ui: &mut Ui, p: &Palette, app: &AppProfile, action: &mut Action) {
        ui.horizontal(|ui| {
            crate::ui::letter_avatar(ui, &app.name, p, 16.0);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(&app.name)
                    .font(font::title())
                    .color(p.text_primary),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Drawn, not a glyph. egui's bundled fonts have no U+2715 MULTIPLICATION X,
                // so a text close button renders as tofu - and a symbol that only appears
                // when the font happens to have it is not a symbol, it is a coincidence.
                let (rect, x) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
                let c = rect.center();
                let arm = 4.0;
                let tint = if x.hovered() { p.text_primary } else { p.text_tertiary };
                let stroke = egui::Stroke::new(1.2, tint);
                let painter = ui.painter();
                painter.line_segment(
                    [c - egui::vec2(arm, arm), c + egui::vec2(arm, arm)],
                    stroke,
                );
                painter.line_segment(
                    [c + egui::vec2(arm, -arm), c - egui::vec2(arm, -arm)],
                    stroke,
                );
                if x.clicked() {
                    *action = Action::GiveUpFocus;
                }
            });
        });
        ui.add_space(6.0);
    }

    fn search_bar(&mut self, ui: &mut Ui, p: &Palette, app: &AppProfile, action: &mut Action) {
        let te = egui::TextEdit::singleline(&mut self.query)
            .hint_text("Search")
            .font(font::command())
            .text_color(p.text_primary)
            .desired_width(ui.available_width() - 18.0)
            .align(egui::Align2::LEFT_CENTER);
        let resp = ui.add(te);

        // Click into the field: the one deliberate focus change in the product.
        if resp.clicked() && !self.focused {
            *action = Action::WantFocus;
        }
        if self.focused && !resp.has_focus() {
            resp.request_focus();
        }

        let active = self.focused || resp.has_focus();
        let w = resp.rect.width().max(1.0);
        let y = resp.rect.bottom() + 2.0;
        ui.painter().line_segment(
            [Pos2::new(resp.rect.min.x, y), Pos2::new(resp.rect.min.x + w, y)],
            Stroke::new(if active { 2.0 } else { 1.0 }, if active { p.accent } else { p.border_strong }),
        );
        ui.add_space(8.0);
        let _ = app;
    }

    fn list(&mut self, ui: &mut Ui, p: &Palette, app: &AppProfile, action: &mut Action) {
        let hover = hover_fill(p);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(
                (size::OVERLAY_MAX_HEIGHT
                    - size::TITLE_BAR_HEIGHT
                    - size::SEARCH_HEIGHT
                    - size::PADDING * 4.0)
                    .max(120.0),
            )
            .show(ui, |ui| {
                // Rows are taken out of self for the loop: draw_row needs &mut self (it moves
                // the selection on hover) and cannot borrow the same Vec immutably.
                let rows = std::mem::take(&mut self.rows);
                if rows.is_empty() {
                    ui.add_space(18.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(if self.query.is_empty() {
                                "No shortcuts listed for this app yet"
                            } else {
                                "Nothing matches that search"
                            })
                            .font(font::command())
                            .color(p.text_secondary),
                        );
                        if !self.query.is_empty() {
                            ui.label(
                                egui::RichText::new("Add your own in apps.json")
                                    .font(font::header())
                                    .color(p.text_tertiary),
                            );
                        }
                    });
                    self.rows = rows;
                    return;
                }

                let mut last_collection: Option<usize> = None;
                for (idx, row) in rows.iter().enumerate() {
                    if Some(row.collection) != last_collection {
                        last_collection = Some(row.collection);
                        ui.add_space(if idx == 0 { 0.0 } else { size::SECTION_GAP });
                        ui.label(
                            egui::RichText::new(&app.collections[row.collection].name)
                                .font(font::header())
                                .color(p.text_secondary),
                        );
                        ui.add_space(2.0);
                    }
                    self.draw_row(ui, p, hover, row, idx, action);
                }
                self.rows = rows;
            });

        if self.pending_enter {
            self.pending_enter = false;
            if let Some(c) = self.selected_combo() {
                *action = Action::Fire(c);
            }
        }
    }

    fn draw_row(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        hover: Color32,
        row: &Row,
        idx: usize,
        action: &mut Action,
    ) {
        let desired = Vec2::new(ui.available_width(), size::ROW_HEIGHT);
        let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
        if !ui.is_rect_visible(rect) {
            return;
        }

        let selected = self.focused && idx == self.selected;
        if selected {
            ui.painter().rect_filled(rect.shrink(1.0), radius::ROW, p.row_selected);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect.shrink(1.0), radius::ROW, hover);
        }

        let key_w = estimate_keycaps_width(&row.keys);
        let (name_rect, key_rect) = crate::ui::split_row(rect, key_w);

        // tokens.json overlay_row: "command name left, keycap cluster right". An explicit
        // left-to-right scope, because `ui.put` inherits the enclosing vertical layout and
        // parked every name against the keycaps - which read as one ragged column of text.
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(name_rect)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&row.name)
                            .font(font::command())
                            .color(p.text_primary),
                    )
                    .truncate(),
                );
            },
        );

        let combo = Combo::parse(&row.keys).ok();
        if let Some(c) = combo {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(key_rect)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = size::KEYCAP_GAP;
                    for k in c.0.iter().rev() {
                        keycap(ui, &k.label(), p, k.is_modifier());
                    }
                },
            );
        }

        if resp.hovered() {
            self.selected = idx;
        }
        if resp.clicked() {
            if let Some(c) = Combo::parse(&row.keys).ok() {
                *action = Action::Fire(c);
            }
        }
    }
}

/// Snapshot of the keys the platform layer saw this frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyFlags {
    pub up: bool,
    pub down: bool,
    pub enter: bool,
    pub escape: bool,
}

/// Cheap width model so the name column truncates before the keycaps are drawn.
fn estimate_keycaps_width(keys: &str) -> f32 {
    let Ok(c) = Combo::parse(keys) else { return 48.0 };
    c.0.iter()
        .map(|k| (k.label().chars().count() as f32 * 7.0 + 12.0).max(size::KEYCAP_MIN_WIDTH))
        .sum::<f32>()
        + size::KEYCAP_GAP * c.0.len().saturating_sub(1) as f32
}
