//! A compact native view using the same playback actions as the main window.

use egui::{Align, Color32, Frame, Layout, Rect, UiBuilder, pos2, vec2};

use crate::{app::App, i18n::gettext, theme};

pub(super) const HEADER_INSET: f32 = 16.0;
pub(super) const HEADER_COVER_SIZE: f32 = 52.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    if app.mini_player.is_some_and(|mini| mini.floating_lyrics) {
        super::lyrics::fullscreen(app, ui);
        return;
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(theme::Palette::dark().window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            super::lyrics::background(app, ui, rect);
            let top = rect.top() + 62.0;
            let side = (rect.width() - 64.0).min(rect.height() - 346.0).max(120.0);
            let cover =
                Rect::from_min_size(pos2(rect.center().x - side / 2.0, top), vec2(side, side));
            let now = app.now_playing();
            if cover_button(ui, cover, &gettext(app.locale, "Floating lyrics")) {
                app.actions.push(crate::model::Action::ToggleMiniLyrics);
            }
            if let Some(now) = &now {
                super::widgets::paint_cover(
                    ui,
                    &theme::Palette::dark(),
                    now.art_url.as_deref().or(now.art_small.as_deref()),
                    cover,
                    12.0,
                    theme::Icon::Music,
                    Some(app.backend.art()),
                );
            }
            let text_rect = Rect::from_min_max(
                pos2(rect.left() + 24.0, cover.bottom() + 16.0),
                pos2(rect.right() - 24.0, cover.bottom() + 76.0),
            );
            let mut text = ui.new_child(
                UiBuilder::new()
                    .max_rect(text_rect)
                    .layout(Layout::top_down(Align::Center)),
            );
            super::lyrics::scrolling_title(
                &mut text,
                now.as_ref().map_or("", |now| now.uri.as_str()),
                now.as_ref().map_or("Spotifast", |now| now.title.as_str()),
                theme::bold(22.0),
                false,
            );
            text.add(
                egui::Label::new(
                    egui::RichText::new(
                        now.as_ref()
                            .map(|now| now.subtitle.clone())
                            .unwrap_or_else(|| gettext(app.locale, "Nothing playing").into_owned()),
                    )
                    .font(theme::regular(14.0))
                    .color(Color32::from_gray(210)),
                )
                .truncate()
                .selectable(true),
            );
            controls(app, ui, rect);
            super::queue::lyrics_overlay(app, ui, rect);
        });
}

pub(super) fn controls(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let id = ui.id().with("mini-controls-reveal");
    if super::queue::covers_mini_controls(app, ui.ctx()) {
        ui.ctx().animate_bool_with_time(id, false, 0.16);
        ui.ctx().data_mut(|data| data.insert_temp(id, 0.0_f32));
        return;
    }
    let previous = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(id))
        .unwrap_or(0.0);
    let hot = Rect::from_min_max(
        pos2(rect.left(), rect.bottom() - (52.0 + 152.0 * previous)),
        rect.max,
    );
    let album_view = app.mini_player.is_some_and(|mini| !mini.floating_lyrics);
    let reveal = album_view
        || super::ivlyrics::toolbar_revealed(ui, hot)
        || (ui.input(|input| input.pointer.any_down()) && previous > 0.0);
    let progress = if album_view {
        1.0
    } else {
        ui.ctx().animate_bool_with_time_and_easing(
            id,
            reveal,
            0.16,
            egui::emath::easing::cubic_in_out,
        )
    };
    ui.ctx().data_mut(|data| data.insert_temp(id, progress));
    let top = rect.bottom() - 52.0 - 152.0 * progress;
    let visible = Rect::from_min_max(pos2(rect.left(), top), rect.max);
    // Overlay the lyrics without changing their viewport or scroll anchor.
    let mut overlay = ui.new_child(UiBuilder::new().max_rect(visible));
    overlay.set_clip_rect(visible.intersect(ui.clip_rect()));
    overlay.interact(visible, id.with("backdrop"), egui::Sense::click());
    super::lyrics::background(app, &mut overlay, rect);
    let controls = Rect::from_min_size(
        pos2(rect.left() + 20.0, top),
        vec2(rect.width() - 40.0, 192.0),
    );
    super::lyrics::fullscreen_controls(app, &mut overlay, controls);
}

pub(super) fn cover_button(ui: &mut egui::Ui, rect: Rect, label: &str) -> bool {
    let response = ui.interact(
        rect,
        ui.id().with("mini-cover-toggle"),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    theme::focus_ring(ui, &response);
    response.on_hover_text(label).clicked()
}
