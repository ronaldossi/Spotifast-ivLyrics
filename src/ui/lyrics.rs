//! The words of the playing track, in a side panel that follows the song.

use egui::{Align, Color32, Frame, Layout, Margin, Rect, Sense, UiBuilder, pos2, vec2};

use crate::app::App;
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Loadable};
use crate::theme::{self, Icon};

use super::widgets;

const LINE_SIZE: f32 = 19.0;
/// Where the line being sung sits, as a fraction of the visible lyrics from
/// the top: high up, so the lines to come fill most of the view.
const SUNG_LINE_AT: f32 = 0.2;

/// Scrolls so the middle of `line` sits `SUNG_LINE_AT` of the way down the
/// visible lyrics.
fn show_sung_line(
    ui: &egui::Ui,
    line: Rect,
    anchor: f32,
    animation: Option<egui::style::ScrollAnimation>,
) {
    let above = (ui.clip_rect().height() * anchor - line.height() / 2.0).max(0.0);
    let target = Rect::from_min_max(pos2(line.left(), line.top() - above), line.max);
    match animation {
        Some(animation) => ui.scroll_to_rect_animation(target, Some(Align::Min), animation),
        None => ui.scroll_to_rect(target, Some(Align::Min)),
    }
}
/// How long a line takes to light up or fade.
const LIGHT_UP_SECONDS: f32 = 0.22;

fn blend(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from(egui::Rgba::from(from) * (1.0 - t) + egui::Rgba::from(to) * t)
}

pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let fit = super::yielding_panel(
        ui.ctx(),
        "lyrics-panel",
        theme::SIDE_PANEL_MIN_WIDTH..=640.0,
        app.settings.lyrics_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()),
    );
    let panel = egui::Panel::right("lyrics-panel")
        .resizable(true)
        .default_size(app.settings.lyrics_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(12, 12)),
        );
    let response = panel.show(ui, |ui| {
        let window_controls = super::window_controls_reservation(
            ui.ctx(),
            app.show_queue_panel,
            app.show_lyrics_panel,
            ui.available_width(),
        );
        ui.add_space(window_controls.lyrics_top);
        super::playback_panel_tabs(app, ui);
        ui.horizontal(|ui| {
            let loaded = matches!(&app.lyrics, Loadable::Loaded(Some(_)));
            if loaded
                && !app.lyrics_following
                && theme::pill_button(
                    ui,
                    &palette,
                    &pgettext(app.locale, "lyrics", "Follow"),
                    false,
                )
                .clicked()
            {
                app.lyrics_following = true;
                app.lyrics_line_shown = None;
            }
        });
        ui.add_space(8.0);
        super::ivlyrics::toolbar(app, ui);
        ui.add_space(8.0);
        contents(app, ui);
    });
    let current_width = response.response.rect.width();
    if (app.settings.lyrics_width - current_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "lyrics-panel", &fit)
    {
        app.settings.lyrics_width = current_width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn contents(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Mic,
            &gettext(app.locale, "Nothing playing"),
            &gettext(app.locale, "Play a song to see its lyrics."),
        );
        return;
    };
    let lyrics = match &app.lyrics {
        Loadable::NotLoaded | Loadable::Loading => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            let message = gettext(
                app.locale,
                // Translators: Keep {error} unchanged. It is the original failure detail.
                "Couldn't fetch the lyrics: {error}",
            )
            .replace("{error}", error);
            ui.add_space(8.0);
            theme::text(ui, message, theme::regular(13.0), palette.secondary);
            ui.add_space(8.0);
            if theme::pill_button(ui, &palette, &gettext(app.locale, "Try again"), false).clicked()
            {
                app.request_lyrics();
            }
            return;
        }
        Loadable::Loaded(None) => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Mic,
                &gettext(app.locale, "No lyrics"),
                &gettext(app.locale, "No lyrics found for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) if lyrics.instrumental => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Music,
                &gettext(app.locale, "Instrumental"),
                &gettext(app.locale, "No timed lyrics for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) => lyrics.clone(),
    };

    let active = app
        .settings
        .ivlyrics
        .active_line(&now.uri, &lyrics, now.position_ms);
    let viewport = ui.available_size();
    let resized = ui.ctx().data_mut(|data| {
        let id = egui::Id::new("lyrics-panel-size");
        let previous = data.get_temp::<egui::Vec2>(id);
        data.insert_temp(id, viewport);
        previous != Some(viewport)
    });
    let follow = app.lyrics_following && (app.lyrics_line_shown != Some(active) || resized);
    // The line being sung is bold and in the accent colour; every other
    // line is quiet, regular text, the same before and after it has been
    // sung. A line takes 220 ms to light up or fade, as in omarchy-lyrics.
    let quiet = palette
        .text
        .gamma_multiply(app.settings.ivlyrics.display.validated().inactive_opacity);
    let fit = super::ivlyrics::panel_scale(app, viewport);
    let size = if fit.is_some() { 32.0 } else { LINE_SIZE };
    let scroll = crate::autoscroll::show(
        ui,
        egui::ScrollArea::vertical()
            .id_salt("lyrics-scroll")
            .auto_shrink([false, false]),
        egui::Vec2b::new(false, true),
        |ui| {
            // Before the first line there is nothing to highlight, so the
            // panel sits at the top rather than wherever it was left.
            if follow && lyrics.synced && active.is_none() {
                let top = ui.cursor().min;
                ui.scroll_to_rect(
                    egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
                    Some(Align::Min),
                );
            }
            ui.add_space(12.0);
            for (index, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(index);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("lyric-line").with(index),
                    is_active,
                    LIGHT_UP_SECONDS,
                );
                let color = blend(quiet, palette.accent, lit);
                let font = if lit > 0.5 {
                    theme::bold(size)
                } else {
                    theme::regular(size)
                };
                let response = ui
                    .scope_builder(
                        UiBuilder::new().layout(Layout::top_down(super::ivlyrics::alignment(app))),
                        |ui| {
                            super::ivlyrics::line(
                                app,
                                ui,
                                &lyrics,
                                index,
                                font,
                                color,
                                super::ivlyrics::LineLayout {
                                    fit,
                                    ..Default::default()
                                },
                            )
                        },
                    )
                    .inner;
                crate::autoscroll::row(ui, &response);
                let rect = response.rect;
                if lyrics.synced
                    && now.playing
                    && response.clicked()
                    && let Some(at_ms) = line.at_ms
                {
                    app.actions.push(Action::Seek(
                        app.settings.ivlyrics.seek_position(&now.uri, at_ms).min(
                            if now.duration_ms > 0 {
                                now.duration_ms
                            } else {
                                u32::MAX
                            },
                        ),
                    ));
                    app.lyrics_following = true;
                }
                if is_active && follow {
                    show_sung_line(ui, rect, SUNG_LINE_AT, None);
                }
                ui.add_space(
                    app.settings.ivlyrics.display.validated().line_spacing * fit.unwrap_or(1.0),
                );
            }
            // Words without timing can only be followed by the clock: sit
            // at the part of the text the song is probably at.
            if app.lyrics_following && !lyrics.synced && now.duration_ms > 0 {
                let fraction =
                    (f64::from(now.position_ms) / f64::from(now.duration_ms)).clamp(0.0, 1.0);
                let content = ui.min_rect();
                let y = content.top() + content.height() * fraction as f32;
                ui.scroll_to_rect(
                    egui::Rect::from_min_max(
                        egui::pos2(content.left(), y),
                        egui::pos2(content.right(), y + 1.0),
                    ),
                    Some(Align::Center),
                );
            }
            // Room for the last line to rise to where a sung line sits.
            ui.add_space((ui.clip_rect().height() * (1.0 - SUNG_LINE_AT)).max(60.0));
        },
    );
    crate::autoscroll::lyrics(ui, scroll.id);
    // Scrolling by hand means the reader wants to look elsewhere; the
    // Follow button in the header picks the song back up.
    if ui.rect_contains_pointer(scroll.inner_rect)
        && ui.input(|input| input.smooth_scroll_delta.y != 0.0)
    {
        app.lyrics_following = false;
    }
    app.lyrics_line_shown = Some(active);
}

pub fn fullscreen(app: &mut App, ui: &mut egui::Ui) {
    egui::CentralPanel::default()
        .frame(Frame::new().fill(theme::Palette::dark().window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            background(app, ui, rect);
            let columns = two_columns(app, rect.size());
            let changed = ui.ctx().data_mut(|data| {
                let id = egui::Id::new("lyrics-two-columns");
                let prior = data.get_temp::<bool>(id);
                data.insert_temp(id, columns);
                prior.is_some_and(|prior| prior != columns)
            });
            if changed {
                app.lyrics_line_shown = None;
            }
            let top = if app.mini_player.is_some() {
                super::mini_player::HEADER_INSET
            } else {
                theme::titlebar_inset(ui.ctx()) + 24.0
            };
            if app.now_playing().is_some() && columns {
                with_cover(app, ui, rect, top);
                super::queue::lyrics_overlay(app, ui, rect);
                return;
            }
            let width = fullscreen_content_width(rect.width());
            let controls_height = if app.mini_player.is_some() {
                52.0
            } else if app.settings.ivlyrics.display.show_controls {
                204.0
            } else {
                12.0
            };
            let region = Rect::from_min_max(
                pos2(rect.center().x - width / 2.0, rect.top() + top),
                pos2(
                    rect.center().x + width / 2.0,
                    rect.bottom() - controls_height,
                ),
            );
            let mut content = ui.new_child(UiBuilder::new().max_rect(region));
            if app.mini_player.is_none() {
                fullscreen_header(app, &mut content);
                content.add_space(20.0);
            }
            track_heading(app, &mut content);
            content.add_space(16.0);
            fullscreen_contents(app, &mut content, None);
            if app.mini_player.is_some() {
                super::mini_player::controls(app, ui, rect);
            } else {
                fullscreen_controls(
                    app,
                    ui,
                    Rect::from_min_max(
                        pos2(region.left(), rect.bottom() - 204.0),
                        pos2(region.right(), rect.bottom() - 12.0),
                    ),
                );
            }
            super::queue::lyrics_overlay(app, ui, rect);
        });
}

/// The narrowest window that shows the cover beside the lyrics; narrower
/// ones keep a single column with a small cover in the heading.
const COVER_BESIDE_MIN_WIDTH: f32 = 900.0;

fn two_columns(app: &App, size: egui::Vec2) -> bool {
    app.settings.ivlyrics.display.two_columns
        && size.x >= COVER_BESIDE_MIN_WIDTH
        && (app.mini_player.is_none() || (size.y >= 560.0 && size.x >= size.y * 1.4))
}

/// Full screen with the cover large: beside the lyrics when there are
/// words to follow, and alone in the middle when there are none, a calm
/// view of what is playing.
fn with_cover(app: &mut App, ui: &mut egui::Ui, rect: Rect, top: f32) {
    let display = app.settings.ivlyrics.display.validated();
    let outer = Rect::from_min_max(
        pos2(rect.left() + 48.0, rect.top() + top),
        pos2(rect.right() - 48.0, rect.bottom() - 40.0),
    );
    let mut header = ui.new_child(UiBuilder::new().max_rect(outer));
    if app.mini_player.is_none() {
        fullscreen_header(app, &mut header);
    } else {
        header.add_space(46.0);
    }
    let below = Rect::from_min_max(
        pos2(outer.left(), header.min_rect().bottom() + 24.0),
        outer.max,
    );
    // The cover moves aside only for words to read. While they load it
    // stays in the middle, so a song that turns out to have none never
    // moves at all.
    let words = matches!(&app.lyrics, Loadable::Loaded(Some(lyrics)) if !lyrics.instrumental);
    if words {
        let mut below = below;
        let under_album = display.controls_below_album;
        let controls_shown = display.show_controls || app.mini_player.is_some();
        if controls_shown && !under_album {
            below.max.y -= 204.0;
        }
        let half = below.width() / 2.0;
        let info_height = if display.show_info {
            display.title_size * 1.3
                + display.artist_size * 1.3
                + 40.0
                + if display.show_album_name { 24.0 } else { 0.0 }
        } else {
            0.0
        };
        let controls_height = if controls_shown && under_album {
            204.0
        } else {
            0.0
        };
        let side = display
            .album_size
            .min(half - 64.0)
            .min((below.height() - info_height - controls_height).max(0.0));
        let cover_height = if display.show_album { side } else { 0.0 };
        let album_center = if display.reversed {
            below.right() - half / 2.0
        } else {
            below.left() + half / 2.0
        };
        let column = Rect::from_center_size(
            pos2(album_center, below.center().y - controls_height / 2.0),
            vec2(side, cover_height + info_height),
        );
        big_cover(app, ui, column, Align::Center);
        let control_width = (column.width() + 72.0).max(360.0).min(half - 24.0);
        let controls = if under_album {
            Rect::from_min_max(
                pos2(
                    column.center().x - control_width / 2.0,
                    column.bottom() + 8.0,
                ),
                pos2(
                    column.center().x + control_width / 2.0,
                    column.bottom() + 204.0,
                ),
            )
        } else {
            Rect::from_center_size(
                pos2(rect.center().x, rect.bottom() - 112.0),
                vec2(480.0, 200.0),
            )
        };
        fullscreen_controls(app, ui, controls);
        let lyrics = if display.reversed {
            Rect::from_min_max(
                pos2(below.left() + 16.0, below.top()),
                pos2(below.center().x - 40.0, below.bottom()),
            )
        } else {
            Rect::from_min_max(
                pos2(below.center().x + 40.0, below.top()),
                pos2(below.right() - 16.0, below.bottom()),
            )
        };
        let mut content = ui.new_child(UiBuilder::new().max_rect(lyrics));
        let clear_y = egui::Rangef::new(
            column.top().max(lyrics.top()),
            if controls_shown && under_album {
                controls.bottom()
            } else {
                column.bottom()
            }
            .min(lyrics.bottom()),
        );
        fullscreen_contents(app, &mut content, Some(clear_y));
    } else {
        let info_height = if display.show_info {
            display.title_size * 1.3
                + display.artist_size * 1.3
                + 40.0
                + if display.show_album_name { 24.0 } else { 0.0 }
        } else {
            0.0
        };
        let side = display
            .album_size
            .min(below.width() * 0.5)
            .min((below.height() - info_height - 288.0).max(0.0));
        let cover_height = if display.show_album { side } else { 0.0 };
        let column = Rect::from_center_size(
            below.center() - vec2(0.0, 80.0),
            vec2(side, cover_height + info_height),
        );
        fullscreen_controls(
            app,
            ui,
            Rect::from_center_size(
                pos2(below.center().x, below.bottom() - 76.0),
                vec2(440.0, 200.0),
            ),
        );
        big_cover(app, ui, column, Align::Center);
        // Why there are no words, quietly, under the song, or that they
        // are still being fetched.
        let (heading, detail) = match &app.lyrics {
            Loadable::Loaded(Some(_)) => (
                gettext(app.locale, "Instrumental"),
                gettext(app.locale, "No timed lyrics for this track."),
            ),
            Loadable::Loaded(None) => (
                gettext(app.locale, "No lyrics"),
                gettext(app.locale, "No lyrics found for this track."),
            ),
            Loadable::Failed(error) => (
                gettext(
                    app.locale,
                    // Translators: Keep {error} unchanged. It is the original failure detail.
                    "Couldn't fetch the lyrics: {error}",
                )
                .replace("{error}", error)
                .into(),
                Default::default(),
            ),
            Loadable::NotLoaded | Loadable::Loading => {
                (gettext(app.locale, "Loading…"), Default::default())
            }
        };
        let heading = ui.painter().text(
            pos2(column.center().x, column.bottom() + 8.0),
            egui::Align2::CENTER_TOP,
            heading,
            theme::semibold(13.0),
            Color32::from_gray(200),
        );
        if matches!(app.lyrics, Loadable::Failed(_)) {
            let retry = Rect::from_center_size(
                pos2(column.center().x, heading.bottom() + 24.0),
                vec2(column.width(), 32.0),
            );
            let mut retry_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(retry)
                    .layout(Layout::top_down(Align::Center)),
            );
            let label = gettext(app.locale, "Try again");
            if theme::pill_button(&mut retry_ui, &theme::Palette::dark(), &label, false).clicked() {
                app.actions.push(Action::RetryLyrics);
            }
            return;
        }
        ui.painter().text(
            pos2(column.center().x, heading.bottom() + 4.0),
            egui::Align2::CENTER_TOP,
            detail,
            theme::regular(13.0),
            Color32::from_gray(170),
        );
    }
}

/// The playing song's cover filling the top of `column`, with its title and
/// artists beneath, aligned to its left edge or centred.
fn big_cover(app: &mut App, ui: &mut egui::Ui, column: Rect, align: Align) {
    let Some(now) = app.now_playing() else {
        return;
    };
    let display = app.settings.ivlyrics.display.validated();
    let side = column.width();
    let cover = Rect::from_min_size(
        column.min,
        vec2(side, if display.show_album { side } else { 0.0 }),
    );
    let radius = display.album_radius;
    if display.show_album {
        if app.mini_player.is_some()
            && super::mini_player::cover_button(
                ui,
                cover,
                &gettext(app.locale, "Show album artwork"),
            )
        {
            app.actions.push(Action::ToggleMiniLyrics);
        }
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 10],
                blur: 88,
                spread: 0,
                color: Color32::from_black_alpha(58),
            }
            .as_shape(cover, radius),
        );
        widgets::paint_cover(
            ui,
            &theme::Palette::dark(),
            now.art_url.as_deref().or(now.art_small.as_deref()),
            cover,
            radius,
            Icon::Music,
            Some(app.backend.art()),
        );
    }
    if !display.show_info {
        return;
    }
    let text_width = (ui.max_rect().width() * 0.5 - 96.0).max(column.width());
    let words = Rect::from_min_max(
        pos2(
            column.center().x - text_width * 0.5,
            cover.bottom() + if display.show_album { 24.0 } else { 0.0 },
        ),
        pos2(column.center().x + text_width * 0.5, column.bottom()),
    );
    let mut text = ui.new_child(
        UiBuilder::new()
            .max_rect(words)
            .layout(Layout::top_down(align)),
    );
    text.spacing_mut().item_spacing.y = 8.0;
    scrolling_title(
        &mut text,
        &now.uri,
        &now.title,
        theme::bold(display.title_size),
        app.lyrics_fullscreen.is_some() && app.mini_player.is_none(),
    );
    text.add(
        egui::Label::new(
            egui::RichText::new(&now.subtitle)
                .font(theme::medium(display.artist_size))
                .color(Color32::from_gray(225)),
        )
        .truncate()
        .selectable(true),
    );
    if display.show_album_name {
        text.add(
            egui::Label::new(
                egui::RichText::new(&now.album_name)
                    .font(theme::regular(14.0))
                    .color(Color32::from_gray(165)),
            )
            .truncate()
            .selectable(true),
        );
    }
}

pub(super) fn fullscreen_controls(app: &mut App, ui: &mut egui::Ui, region: Rect) {
    if super::queue::covers_mini_controls(app, ui.ctx()) {
        return;
    }
    if !app.settings.ivlyrics.display.show_controls && app.mini_player.is_none() {
        return;
    }
    let palette = theme::Palette::dark();
    let now = app.now_playing();
    let transport = Rect::from_min_size(region.min, vec2(region.width(), 128.0));
    super::player_bar::transport(app, ui, now.as_ref(), transport, &palette);
    if app.settings.ivlyrics.display.show_volume || app.mini_player.is_some() {
        let width = (region.width() * 0.62).clamp(150.0, 230.0);
        let row = Rect::from_center_size(
            pos2(region.center().x - 19.0, region.top() + 134.0),
            vec2(width + 38.0, 30.0),
        );
        let mut volume = ui.new_child(
            UiBuilder::new()
                .max_rect(row)
                .layout(Layout::right_to_left(Align::Center)),
        );
        volume.spacing_mut().item_spacing.x = 8.0;
        super::player_bar::volume(app, &mut volume, now.as_ref(), &palette, width);
    }
    let saved = now
        .as_ref()
        .is_some_and(|now| app.is_saved(&now.uri).is_some());
    let width = if saved {
        29.0 + 4.0 * 30.0 + 4.0 * 14.0
    } else {
        4.0 * 30.0 + 3.0 * 14.0
    } + if app.mini_player.is_some() { 44.0 } else { 0.0 };
    let extras = Rect::from_center_size(
        pos2(region.center().x, region.bottom() - 18.0),
        vec2(width, 30.0),
    );
    let mut controls = ui.new_child(
        UiBuilder::new()
            .max_rect(extras)
            .layout(Layout::right_to_left(Align::Center)),
    );
    super::player_bar::extras(app, &mut controls, now.as_ref(), &palette);
    if let Some(now) = &now
        && let Some(saved) = app.is_saved(&now.uri)
        && theme::icon_button(
            &mut controls,
            if saved {
                Icon::HeartFilled
            } else {
                Icon::Heart
            },
            17.0,
            if saved {
                palette.accent
            } else {
                palette.secondary
            },
            palette.text,
            &if saved {
                gettext(app.locale, "Remove from Liked Songs")
            } else {
                gettext(app.locale, "Save to Liked Songs")
            },
        )
        .clicked()
    {
        app.actions.push(Action::ToggleSaved(now.uri.clone()));
    }
}

fn fullscreen_content_width(viewport_width: f32) -> f32 {
    let available = (viewport_width - 48.0).max(0.0);
    (viewport_width * 0.72).clamp(400.0, 960.0).min(available)
}

fn preferred_backdrop_art(small: Option<String>, large: Option<String>) -> Option<String> {
    small.or(large)
}

pub(super) fn background(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    background_region(app, ui, rect, rect, 0.0);
}

pub(super) fn background_region(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    region: Rect,
    radius: f32,
) {
    theme::apply_local(ui, &theme::Palette::dark());
    let art = app
        .now_playing()
        .and_then(|now| preferred_backdrop_art(now.art_small, now.art_url));
    let display = app.settings.ivlyrics.display.validated();
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(region, radius, theme::Palette::dark().window);
    if display.album_art_background
        && let Some(texture) =
            app.lyrics_backdrop
                .texture(ui.ctx(), app.backend.art(), art.as_deref())
    {
        let ratio = (rect.width() / rect.height().max(1.0))
            / (texture.size_vec2().x / texture.size_vec2().y.max(1.0));
        let size = if ratio > 1.0 {
            vec2(1.0, 1.0 / ratio)
        } else {
            vec2(ratio, 1.0)
        };
        let mapping =
            egui::emath::RectTransform::from_to(rect, Rect::from_center_size(pos2(0.5, 0.5), size));
        let uv = Rect::from_min_max(
            mapping.transform_pos(region.min),
            mapping.transform_pos(region.max),
        );
        painter.add(
            egui::epaint::RectShape::filled(region, radius, Color32::from_gray(180))
                .with_texture(texture.id(), uv),
        );
    }
    painter.rect_filled(
        region,
        radius,
        Color32::from_black_alpha((display.background_dim * 255.0) as u8),
    );
    if radius > 0.0 {
        let shade = ((region.center().y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        painter.rect_filled(
            region,
            radius,
            Color32::from_black_alpha((95.0 * shade) as u8),
        );
    } else {
        widgets::paint_vertical_gradient(
            ui,
            rect,
            Color32::TRANSPARENT,
            Color32::from_black_alpha(95),
        );
    }
}

fn fullscreen_header(app: &mut App, ui: &mut egui::Ui) {
    let palette = theme::Palette::dark();
    let region = ui.available_rect_before_wrap();
    let visible = super::ivlyrics::toolbar_revealed(
        ui,
        Rect::from_min_size(region.min, vec2(region.width(), 84.0)),
    );
    let mut controls = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(
        region.min,
        pos2(region.right() - 42.0, region.bottom()),
    )));
    if !visible {
        controls.set_invisible();
    }
    super::ivlyrics::toolbar(app, &mut controls);
    let close = Rect::from_min_size(pos2(region.right() - 30.0, region.top()), vec2(30.0, 30.0));
    let mut exit = ui.new_child(UiBuilder::new().max_rect(close));
    if !visible {
        exit.set_invisible();
    }
    if theme::icon_button(
        &mut exit,
        Icon::Shrink,
        18.0,
        palette.secondary,
        palette.text,
        &gettext(app.locale, "Leave full screen (Esc)"),
    )
    .clicked()
    {
        app.actions.push(Action::SetLyricsFullscreen(false));
    }
    // Keep the album and lyric viewport anchored when manual scrolling reveals Follow.
    controls.scope(|ui| {
        if app.lyrics_following {
            ui.set_invisible();
        }
        if theme::pill_button(
            ui,
            &palette,
            &pgettext(app.locale, "lyrics", "Follow"),
            false,
        )
        .clicked()
        {
            app.actions.push(Action::FollowLyrics);
        }
    });
    ui.advance_cursor_after_rect(controls.min_rect().union(close));
}

fn track_heading(app: &mut App, ui: &mut egui::Ui) {
    let display = app.settings.ivlyrics.display.validated();
    if !display.show_info && app.mini_player.is_none() {
        return;
    }
    if let Some(now) = app.now_playing() {
        ui.horizontal(|ui| {
            if display.show_album || app.mini_player.is_some() {
                let size = super::mini_player::HEADER_COVER_SIZE;
                let floating = app.mini_player.is_some_and(|mini| mini.floating_lyrics);
                let (rect, response) = ui.allocate_exact_size(
                    vec2(size, size),
                    if floating {
                        Sense::click()
                    } else {
                        Sense::hover()
                    },
                );
                if floating {
                    let label = gettext(app.locale, "Show album artwork");
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            label.as_ref(),
                        )
                    });
                    theme::focus_ring(ui, &response);
                    if response.on_hover_text(label).clicked() {
                        app.actions.push(Action::ToggleMiniLyrics);
                    }
                }
                widgets::paint_cover(
                    ui,
                    &theme::Palette::dark(),
                    now.art_small.as_deref().or(now.art_url.as_deref()),
                    rect,
                    4.0,
                    Icon::Music,
                    Some(app.backend.art()),
                );
            }
            ui.vertical(|ui| {
                scrolling_title(
                    ui,
                    &now.uri,
                    &now.title,
                    theme::semibold(22.0),
                    app.lyrics_fullscreen.is_some() && app.mini_player.is_none(),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&now.subtitle)
                            .font(theme::regular(13.0))
                            .color(Color32::from_gray(235)),
                    )
                    .truncate()
                    .selectable(true),
                );
            });
        });
    }
}

pub(super) fn scrolling_title(
    ui: &mut egui::Ui,
    uri: &str,
    title: &str,
    mut font: egui::FontId,
    fullscreen: bool,
) {
    let width = ui.available_width().max(0.0);
    let fixed = fullscreen && title.chars().count() <= 30;
    let mut galley = crate::bidi::layout_line(ui.painter(), title, font.clone(), Color32::WHITE);
    if fixed && width > 0.0 {
        // Re-layout after scaling because font hinting can change the measured width.
        while galley.size().x > width && font.size > 1.0 {
            font.size = (font.size * width / galley.size().x - 0.5).max(1.0);
            galley = crate::bidi::layout_line(ui.painter(), title, font.clone(), Color32::WHITE);
        }
    }
    let overflow = if fixed {
        0.0
    } else {
        (galley.size().x - width).max(0.0)
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    let id = ui.id().with("scrolling-title");
    let content = egui::Id::new((uri, title, width.to_bits(), galley.size().x.to_bits()));
    let time = ui.input(|input| input.time);
    let paused = ui.rect_contains_pointer(rect) || ui.input(|input| input.pointer.primary_down());
    let elapsed = ui.ctx().data_mut(|data| {
        let state = data.get_temp_mut_or_insert_with(id, || (content, time, None::<f64>));
        if state.0 != content || overflow == 0.0 {
            *state = (content, time, None);
        }
        if paused {
            state.2.get_or_insert(time);
        } else if let Some(since) = state.2.take() {
            state.1 += time - since;
        }
        state.2.unwrap_or(time) - state.1
    });
    let offset = title_scroll_offset(elapsed, overflow);
    let align = if overflow == 0.0 {
        match ui.layout().horizontal_align() {
            Align::Center => (width - galley.size().x) / 2.0,
            Align::Max => width - galley.size().x,
            Align::Min => 0.0,
        }
    } else {
        0.0
    };
    let mut text = ui.new_child(
        UiBuilder::new()
            .id_salt("scrolling-title-text")
            .max_rect(Rect::from_min_size(
                rect.min + vec2(align - offset, 0.0),
                galley.size(),
            ))
            .layout(Layout::top_down(Align::Min)),
    );
    text.set_clip_rect(rect.intersect(ui.clip_rect()));
    text.add(egui::Label::new(galley).selectable(true));
    if overflow > 0.0 && !paused && ui.is_rect_visible(rect) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
    }
}

fn title_scroll_offset(elapsed: f64, overflow: f32) -> f32 {
    if overflow <= 0.0 {
        return 0.0;
    }
    let travel = f64::from(overflow) / 22.0;
    let phase = elapsed.max(0.0) % (1.5 + travel + 2.0);
    ((phase - 1.5).max(0.0) * 22.0).min(f64::from(overflow)) as f32
}

fn lyric_viewport_scale(size: egui::Vec2) -> f32 {
    (size.x / 480.0).min(size.y / 600.0).clamp(0.45, 1.0)
}

pub(super) fn fullscreen_contents(app: &mut App, ui: &mut egui::Ui, clear_y: Option<egui::Rangef>) {
    let palette = theme::Palette::dark();
    let compact_mini =
        app.mini_player.is_some() && !two_columns(app, ui.ctx().content_rect().size());
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Mic,
            &gettext(app.locale, "Nothing playing"),
            &gettext(app.locale, "Play a song to see its lyrics."),
        );
        return;
    };
    let lyrics = match &app.lyrics {
        Loadable::NotLoaded | Loadable::Loading => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            let message = gettext(
                app.locale,
                // Translators: Keep {error} unchanged. It is the original failure detail.
                "Couldn't fetch the lyrics: {error}",
            )
            .replace("{error}", error);
            ui.add_space(8.0);
            theme::text(ui, message, theme::regular(13.0), palette.text);
            ui.add_space(8.0);
            if theme::pill_button(ui, &palette, &gettext(app.locale, "Try again"), false).clicked()
            {
                app.actions.push(Action::RetryLyrics);
            }
            return;
        }
        Loadable::Loaded(None) => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Mic,
                &gettext(app.locale, "No lyrics"),
                &gettext(app.locale, "No lyrics found for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) if lyrics.instrumental => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Music,
                &gettext(app.locale, "Instrumental"),
                &gettext(app.locale, "No timed lyrics for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) => lyrics.clone(),
    };

    let active = app
        .settings
        .ivlyrics
        .active_line(&now.uri, &lyrics, now.position_ms);
    // A Follow click resets the remembered line after drawing. Other frames
    // record the shown line before any line-click action restores following.
    if !app.actions.iter().any(|action| {
        matches!(
            action,
            Action::FollowLyrics | Action::IvLyrics(crate::ivlyrics::Action::ToggleAi)
        )
    }) {
        app.actions.push(Action::LyricsLineShown(active));
    }
    let viewport = ui.available_rect_before_wrap();
    let manual_scroll = ui.rect_contains_pointer(viewport)
        && ui.input(|input| {
            input.smooth_scroll_delta.y != 0.0
                || (input.pointer.primary_down() && input.pointer.delta().y != 0.0)
        });
    let following = app.lyrics_following && !manual_scroll;
    let resized = ui.ctx().data_mut(|data| {
        let id = egui::Id::new("lyrics-layout-size");
        let previous = data.get_temp::<egui::Vec2>(id);
        data.insert_temp(id, viewport.size());
        previous != Some(viewport.size())
    });
    let follow = following && (app.lyrics_line_shown != Some(active) || resized);
    let animation = egui::style::ScrollAnimation::duration(0.45);
    let size = 32.0;
    let fit = if app.mini_player.is_some_and(|mini| mini.floating_lyrics) {
        super::ivlyrics::panel_scale(app, viewport.size()).unwrap_or(1.0)
    } else {
        lyric_viewport_scale(viewport.size())
    };
    // The line being sung brightens; all lines keep the same font metrics
    // so highlighting cannot rewrap the words during a transition.
    // A line takes 300 ms to light up or fade.
    let quiet = palette
        .text
        .gamma_multiply(app.settings.ivlyrics.display.validated().inactive_opacity);
    ui.spacing_mut().scroll.fade.strength = 0.0;
    egui::ScrollArea::vertical()
        .id_salt(("fullscreen-lyrics-scroll", &now.uri))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Before the first line there is nothing to highlight, so the
            // panel sits at the top rather than wherever it was left.
            if follow && lyrics.synced && active.is_none() {
                let top = ui.cursor().min;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
                    Some(Align::Min),
                    animation,
                );
            }
            // Room for the first line to sit where a sung line sits, and for
            // the last to rise to it.
            let (padding, below) = if compact_mini {
                (32.0, viewport.height())
            } else if lyrics.synced {
                (
                    (viewport.height() * 0.34 - size).max(12.0),
                    viewport.height() * 0.66,
                )
            } else {
                (12.0, 60.0)
            };
            ui.add_space(padding);
            let mut previous: Option<Rect> = None;
            for (index, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(index);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("lyric-line").with(("fullscreen", &now.uri, index)),
                    is_active,
                    0.3,
                );
                let color = if lyrics.synced {
                    blend(quiet, palette.text, lit)
                } else {
                    palette.text
                };
                let font = theme::bold(size);
                let response = ui
                    .scope_builder(
                        UiBuilder::new().layout(Layout::top_down(super::ivlyrics::alignment(app))),
                        |ui| {
                            super::ivlyrics::line(
                                app,
                                ui,
                                &lyrics,
                                index,
                                font,
                                color,
                                super::ivlyrics::LineLayout {
                                    fit: Some(fit),
                                    clear_y,
                                },
                            )
                        },
                    )
                    .inner;
                let rect = response.rect;
                if lyrics.synced
                    && now.playing
                    && response.clicked()
                    && let Some(at_ms) = line.at_ms
                {
                    app.actions.push(Action::Seek(
                        app.settings.ivlyrics.seek_position(&now.uri, at_ms).min(
                            if now.duration_ms > 0 {
                                now.duration_ms
                            } else {
                                u32::MAX
                            },
                        ),
                    ));
                    app.actions.push(Action::FollowLyrics);
                }
                if is_active && follow {
                    if compact_mini {
                        let top = previous.unwrap_or(rect).min - vec2(0.0, 32.0);
                        ui.scroll_to_rect_animation(
                            Rect::from_min_size(top, vec2(1.0, 1.0)),
                            Some(Align::Min),
                            animation,
                        );
                    } else {
                        show_sung_line(ui, rect, 0.34, Some(animation));
                    }
                }
                previous = Some(rect);
                ui.add_space(app.settings.ivlyrics.display.validated().line_spacing * fit);
            }
            // Words without timing can only be followed by the clock: sit
            // at the part of the text the song is probably at.
            if following && !lyrics.synced && now.duration_ms > 0 {
                let fraction =
                    (f64::from(now.position_ms) / f64::from(now.duration_ms)).clamp(0.0, 1.0);
                let content = ui.min_rect();
                let y = content.top() + content.height() * fraction as f32;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_max(
                        egui::pos2(content.left(), y),
                        egui::pos2(content.right(), y + 1.0),
                    ),
                    Some(Align::Center),
                    animation,
                );
            }
            ui.add_space(below.max(60.0));
        });
    // Scrolling by hand means the reader wants to look elsewhere; the
    // Follow button in the header picks the song back up.
    if manual_scroll && app.lyrics_following {
        app.actions.push(Action::PauseLyricsFollow);
    }
    if now.playing
        && lyrics.synced
        && let Some(next) = lyrics
            .lines
            .iter()
            .filter_map(|line| line.at_ms)
            .map(|at| app.settings.ivlyrics.seek_position(&now.uri, at))
            .find(|at| *at > now.position_ms)
    {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(u64::from(
                next - now.position_ms,
            )));
    }
}

#[cfg(test)]
mod tests {
    use super::{fullscreen_content_width, lyric_viewport_scale, preferred_backdrop_art};

    #[test]
    fn long_titles_wait_scroll_hold_the_end_and_restart() {
        let offset = |time| super::title_scroll_offset(time, 220.0);
        assert_eq!(offset(0.0), 0.0);
        assert_eq!(offset(1.5), 0.0);
        assert_eq!(offset(6.5), 110.0);
        assert_eq!(offset(11.5), 220.0);
        assert_eq!(offset(13.49), 220.0);
        assert_eq!(offset(13.5), 0.0);
        assert_eq!(offset(15.0), 0.0);
        assert_eq!(offset(20.0), 110.0);
        assert_eq!(super::title_scroll_offset(100.0, 0.0), 0.0);
    }

    #[test]
    fn scrolling_title_clips_full_text_pauses_on_hover_and_resets_for_new_content() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let title = "눈 (SNOW) (Feat. 이문세) A long song title with its complete ending";
        let draw = |time, width, uri: &str, text: &str, pointer| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 100.0),
                    )),
                    events: vec![egui::Event::PointerMoved(pointer)],
                    ..Default::default()
                },
                |ui| super::scrolling_title(ui, uri, text, crate::theme::semibold(22.0), false),
            );
            output.textures_delta.clear();
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(painted) if painted.galley.job.text == text => {
                        assert!(shape.clip_rect.width() <= width);
                        Some(painted.pos.x)
                    }
                    _ => None,
                })
                .expect("full, untruncated title is painted")
        };
        let away = egui::pos2(190.0, 90.0);
        let start = draw(0.0, 200.0, "one", title, away);
        assert_eq!(draw(1.0, 200.0, "one", title, away), start);
        let moving = draw(4.0, 200.0, "one", title, away);
        assert!((start - moving - 55.0).abs() < 0.1);
        let held = draw(5.0, 200.0, "one", title, egui::pos2(20.0, 10.0));
        assert_eq!(
            draw(20.0, 200.0, "one", title, egui::pos2(20.0, 10.0)),
            held
        );
        assert_eq!(draw(21.0, 200.0, "one", title, away), held);
        assert_eq!(draw(22.0, 200.0, "two", title, away), start);
        assert_eq!(draw(27.0, 240.0, "two", title, away), start);
        assert_eq!(draw(28.0, 240.0, "two", "Short", away), start);
        assert_eq!(draw(90.0, 240.0, "two", "Short", away), start);
    }

    #[test]
    fn fullscreen_titles_up_to_thirty_characters_fit_one_line_without_moving() {
        for title in [
            "Burn - Remastered 2004".to_string(),
            "가".repeat(29),
            "가".repeat(30),
            "W".repeat(30),
            "가".repeat(31),
        ] {
            for fullscreen in [false, true] {
                let ctx = egui::Context::default();
                crate::theme::install(&ctx);
                let draw = |time| {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(260.0, 100.0),
                            )),
                            events: vec![egui::Event::PointerMoved(egui::pos2(-10.0, -10.0))],
                            ..Default::default()
                        },
                        |ui| {
                            super::scrolling_title(
                                ui,
                                "track",
                                &title,
                                crate::theme::bold(40.0),
                                fullscreen,
                            )
                        },
                    );
                    output.textures_delta.clear();
                    output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == title => {
                                assert_eq!(text.galley.rows.len(), 1);
                                if fullscreen && title.chars().count() <= 30 {
                                    assert!(text.pos.x >= shape.clip_rect.left() - 0.1);
                                    assert!(
                                        text.pos.x + text.galley.size().x
                                            <= shape.clip_rect.right() + 0.1
                                    );
                                }
                                Some(text.pos.x)
                            }
                            _ => None,
                        })
                        .expect("complete title remains available for selection")
                };
                let start = draw(0.0);
                let later = draw(5.0);
                if fullscreen && title.chars().count() <= 30 {
                    assert_eq!(start, later);
                } else {
                    assert!(
                        later < start,
                        "long fullscreen titles and mini titles still scroll"
                    );
                }
            }
        }
    }

    #[test]
    fn lyric_scale_follows_both_dimensions_without_changing_row_ratios() {
        let small = lyric_viewport_scale(egui::vec2(290.0, 400.0));
        let large = lyric_viewport_scale(egui::vec2(580.0, 800.0));
        assert!(small < 0.7 && large == 1.0);
        assert!(lyric_viewport_scale(egui::vec2(580.0, 300.0)) < large);
        assert!(((38.0 * small) / (20.0 * small) - 1.9).abs() < 0.001);
        assert!(((40.0 * small) / (20.0 * small) - 2.0).abs() < 0.001);
    }

    #[test]
    fn fullscreen_backdrop_prefers_small_art_with_large_art_as_fallback() {
        let small = "small".to_string();
        let large = "large".to_string();
        assert_eq!(
            preferred_backdrop_art(Some(small.clone()), Some(large.clone())),
            Some(small)
        );
        assert_eq!(
            preferred_backdrop_art(None, Some(large.clone())),
            Some(large)
        );
        assert_eq!(preferred_backdrop_art(None, None), None);
    }

    #[test]
    fn fullscreen_content_width_never_inverts_a_narrow_viewport() {
        for viewport_width in [0.0, 24.0, 47.0, 48.0, 64.0, 760.0, 2_000.0] {
            let width = fullscreen_content_width(viewport_width);
            assert!(width >= 0.0);
            assert!(width <= (viewport_width - 48.0).max(0.0));
        }
        assert_eq!(fullscreen_content_width(47.0), 0.0);
        assert_eq!(fullscreen_content_width(2_000.0), 960.0);
    }
}
