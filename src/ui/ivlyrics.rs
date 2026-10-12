//! ivLyrics controls shared by the native panel and full-screen view.

use egui::{Color32, Sense};

use crate::app::App;
use crate::i18n::gettext;
use crate::ivlyrics::{Action as IvAction, Secret};
use crate::model::{Action, Loadable};

pub(super) fn toolbar_revealed(ui: &egui::Ui, rect: egui::Rect) -> bool {
    let id = egui::Id::new("lyrics-toolbar-keyboard");
    let mut keyboard = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id))
        .unwrap_or(false);
    ui.input(|input| {
        if input.pointer.delta() != egui::Vec2::ZERO || input.pointer.any_pressed() {
            keyboard = false;
        }
        if input.key_pressed(egui::Key::Tab) {
            keyboard = true;
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, keyboard));
    ui.rect_contains_pointer(rect) || egui::Popup::is_any_open(ui.ctx()) || keyboard
}

pub fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let reveal = toolbar_revealed(
        ui,
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 84.0)),
    );
    ui.scope(|ui| {
    if !reveal { ui.set_invisible(); }
    ui.horizontal_wrapped(|ui| {
        if crate::theme::icon_button(
            ui,
            crate::theme::Icon::Settings,
            18.0,
            ui.visuals().text_color(),
            ui.visuals().strong_text_color(),
            &gettext(locale, "ivLyrics settings"),
        )
        .clicked()
        {
            app.actions.push(Action::IvLyrics(IvAction::Settings(true)));
        }
        let track = app.now_playing().filter(|now| !now.is_episode);
        let selected = app.settings.ivlyrics.provider(track.as_ref().map_or("", |now| now.uri.as_str()));
        let source = selected.map(|source| source.name()).unwrap_or_else(|| match &app.lyrics {
            Loadable::Loaded(Some(lyrics)) => lyrics.source.name(),
            _ => "Lyrics",
        });
        let provider = ui.add(egui::Button::new(if source == "Lyrics" { gettext(locale, "Lyrics").into_owned() } else { source.to_owned() }).frame_when_inactive(false));
        egui::Popup::menu(&provider).id(ui.id().with("lyrics-provider"))
            .frame(egui::Frame::new().fill(app.palette.window)
                .stroke(egui::Stroke::new(1.0, app.palette.outline)).corner_radius(12).inner_margin(18))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                crate::theme::apply_local(ui, &app.palette);
                ui.set_width(320.0);
                ui.heading(gettext(locale, "Lyrics"));
                ui.add_space(8.0);
                for source in [None, Some(crate::lyrics::Source::Alsong), Some(crate::lyrics::Source::Spotify), Some(crate::lyrics::Source::Lrclib)] {
                    let label = source.map(|source| source.name().to_owned()).unwrap_or_else(|| gettext(locale, "Automatic").into_owned());
                    if crate::theme::soft_button(ui, &app.palette, None, &label, selected == source).clicked() {
                        app.actions.push(Action::IvLyrics(IvAction::Provider(source)));
                        ui.close();
                    }
                }
                ui.add_space(8.0);
                ui.small(app.settings.ivlyrics.ordered_providers().iter().map(|source| source.name()).collect::<Vec<_>>().join(" → "));
                if crate::theme::pill_button(ui, &app.palette, &gettext(locale, "ivLyrics settings"), false).clicked() {
                    app.actions.push(Action::IvLyrics(IvAction::Settings(true)));
                    ui.close();
                }
            });
        if selected == Some(crate::lyrics::Source::Alsong)
            || matches!(&app.lyrics, Loadable::Loaded(Some(lyrics)) if lyrics.source == crate::lyrics::Source::Alsong)
        {
            let enabled = track.is_some() && !app.offline;
            let button = ui.add_enabled(enabled, egui::Button::new(gettext(locale, "ALSong")).frame_when_inactive(false));
            button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, gettext(locale, "ALSong candidates").as_ref()));
            if button.on_hover_text(gettext(locale, "ALSong candidates")).clicked() {
                app.actions.push(Action::IvLyrics(IvAction::Candidates(true)));
            }
        }
        if let Some(now) = track {
            let mut offset = app.settings.ivlyrics.offset(&now.uri);
            if ui.scope(|ui| {
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                let response = ui.add(egui::DragValue::new(&mut offset)
                        .range(-60_000..=60_000)
                        .speed(25)
                        .suffix(" ms"));
                if response.dragged() {
                    offset = (offset as f32 / 25.0).round() as i32 * 25;
                }
                response
            }).inner.on_hover_text(gettext(
                    locale,
                    "Lyric timing for this song. Positive values show lyrics later.",
                ))
                .changed()
            {
                app.actions.push(Action::IvLyrics(IvAction::Offset(offset)));
            }
            if offset != 0 && ui.small_button(gettext(locale, "Reset")).clicked() {
                app.actions.push(Action::IvLyrics(IvAction::Offset(0)));
            }
        }
    });
    });
    if let Some(notice) = &app.ivlyrics.notice {
        ui.add(egui::Label::new(egui::RichText::new(notice).small()).wrap());
    }
    if app.settings.ivlyrics.ai_enabled {
        match &app.ivlyrics.translated {
            Loadable::Loading => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(gettext(locale, "Translating lyrics…"));
                });
            }
            Loadable::Failed(error) => {
                ui.add(egui::Label::new(egui::RichText::new(error).small()).wrap());
                if ui
                    .small_button(gettext(locale, "Retry translation"))
                    .clicked()
                {
                    app.actions
                        .push(Action::IvLyrics(IvAction::RetryTranslation));
                }
            }
            _ => {}
        }
    }
}

pub(super) fn alignment(app: &App) -> egui::Align {
    match app.settings.ivlyrics.display.alignment {
        crate::ivlyrics::Alignment::Left => egui::Align::Min,
        crate::ivlyrics::Alignment::Center => egui::Align::Center,
        crate::ivlyrics::Alignment::Right => egui::Align::Max,
    }
}

fn lyric_font(weight: crate::ivlyrics::FontWeight, size: f32) -> egui::FontId {
    use crate::ivlyrics::FontWeight;
    match weight {
        FontWeight::Regular => crate::theme::regular(size),
        FontWeight::Medium => crate::theme::medium(size),
        FontWeight::Semibold => crate::theme::semibold(size),
        FontWeight::Bold => crate::theme::bold(size),
    }
}

fn lyric_layout(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    spacing: f32,
    align: egui::Align,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let galley = crate::bidi::layout(ui.painter(), text, font, color, width, usize::MAX, None);
    let mut job = (*galley.job).clone();
    job.halign = align;
    for section in &mut job.sections {
        section.format.extra_letter_spacing = spacing;
    }
    let mut galley = ui.painter().layout_job(job);
    crate::bidi::reorder(&mut galley);
    galley
}

fn lyric_weight(
    settings: &crate::ivlyrics::Settings,
    source: crate::lyrics::Source,
    rows: usize,
    row: usize,
    secondary: bool,
) -> crate::ivlyrics::FontWeight {
    if source == crate::lyrics::Source::Alsong {
        let custom = match rows {
            2 => settings.two_row_weights.get(row),
            3 => settings.three_row_weights.get(row),
            _ => None,
        };
        if let Some(weight) = custom {
            return *weight;
        }
    }
    if secondary {
        crate::ivlyrics::FontWeight::Regular
    } else {
        settings.font_weight
    }
}

fn weight_picker(
    ui: &mut egui::Ui,
    locale: crate::i18n::Locale,
    weight: &mut crate::ivlyrics::FontWeight,
) {
    use crate::ivlyrics::FontWeight;
    let choices = [
        (FontWeight::Regular, gettext(locale, "Regular")),
        (FontWeight::Medium, gettext(locale, "Medium")),
        (FontWeight::Semibold, gettext(locale, "Semibold")),
        (FontWeight::Bold, gettext(locale, "Bold")),
    ];
    egui::ComboBox::from_id_salt("weight")
        .width(128.0)
        .selected_text(
            choices
                .iter()
                .find(|(value, _)| value == weight)
                .unwrap()
                .1
                .as_ref(),
        )
        .show_ui(ui, |ui| {
            for (value, label) in choices {
                ui.selectable_value(weight, value, label);
            }
        });
}

fn toggle_setting(app: &App, ui: &mut egui::Ui, value: &mut bool, label: &str) {
    super::widgets::setting_row(ui, &app.palette, label, "", |ui| {
        super::widgets::switch(ui, &app.palette, label, value);
    });
}

fn slider_setting(
    app: &App,
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
) {
    ui.push_id(label, |ui| {
        super::widgets::setting_row_sized(ui, &app.palette, label, "", 260.0, |ui| {
            ui.spacing_mut().interact_size.x = 64.0;
            ui.spacing_mut().slider_width = (ui.available_width() - 90.0).clamp(60.0, 160.0);
            let response = super::settings::setting_slider(
                ui,
                &app.palette,
                value,
                range,
                |slider| slider,
                |value| value.max_decimals(2),
            );
            response.widget_info(|| {
                egui::WidgetInfo::slider(ui.is_enabled(), f64::from(*value), label)
            });
        });
    });
}

fn row_type_settings(
    app: &App,
    ui: &mut egui::Ui,
    settings: &mut crate::ivlyrics::Settings,
    rows: usize,
) {
    let title = if rows == 2 {
        gettext(app.locale, "Two-row lyrics (px)")
    } else {
        gettext(app.locale, "Three-row lyrics (px)")
    };
    ui.push_id(rows, |ui| {
        super::settings::section(ui, &app.palette, &title, |ui| {
            let (sizes, weights, gaps): (
                &mut [f32],
                &mut [crate::ivlyrics::FontWeight],
                &mut [f32],
            ) = if rows == 2 {
                (
                    &mut settings.two_row_sizes,
                    &mut settings.two_row_weights,
                    std::slice::from_mut(&mut settings.two_row_spacing),
                )
            } else {
                (
                    &mut settings.three_row_sizes,
                    &mut settings.three_row_weights,
                    &mut settings.three_row_spacing,
                )
            };
            for index in 0..rows {
                ui.push_id(index, |ui| {
                    super::widgets::setting_row_sized(
                        ui,
                        &app.palette,
                        &gettext(app.locale, "Row {row} text size")
                            .replace("{row}", &(index + 1).to_string()),
                        "",
                        400.0,
                        |ui| {
                            let controls = |ui: &mut egui::Ui| {
                                ui.spacing_mut().interact_size.x = 64.0;
                                ui.spacing_mut().slider_width =
                                    (ui.available_width() - 90.0).clamp(60.0, 160.0);
                                super::settings::setting_slider(
                                    ui,
                                    &app.palette,
                                    &mut sizes[index],
                                    0.0..=120.0,
                                    |slider| slider,
                                    |value| value.suffix(" px").max_decimals(0),
                                );
                                weight_picker(ui, app.locale, &mut weights[index]);
                            };
                            if ui.available_width() < 400.0 {
                                ui.vertical(controls);
                            } else {
                                ui.horizontal(controls);
                            }
                        },
                    );
                });
            }
            for (index, gap) in gaps.iter_mut().enumerate() {
                let label = gettext(app.locale, "Gap between rows {first} and {second} (px)")
                    .replace("{first}", &(index + 1).to_string())
                    .replace("{second}", &(index + 2).to_string());
                slider_setting(app, ui, gap, 0.0..=60.0, &label);
            }
            ui.add_space(12.0);
            ui.label(gettext(app.locale, "Preview"));
            let samples = if rows == 2 {
                ["A quiet road beneath the moon", "달 아래 고요한 길", ""]
            } else {
                [
                    "遠くの光をたどって",
                    "토오쿠노 히카리오 타돗테",
                    "먼 곳의 빛을 따라서",
                ]
            };
            egui::Frame::new()
                .fill(app.palette.window)
                .corner_radius(8)
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let normal_gap = ui.spacing().item_spacing.y;
                    for (row, text) in samples.iter().take(rows).enumerate() {
                        ui.spacing_mut().item_spacing.y = settings
                            .row_spacing(crate::lyrics::Source::Alsong, rows, row)
                            .unwrap_or(normal_gap);
                        let size =
                            settings.line_size(crate::lyrics::Source::Alsong, rows, row, 30.0);
                        let weight = lyric_weight(
                            settings,
                            crate::lyrics::Source::Alsong,
                            rows,
                            row,
                            row > 0,
                        );
                        ui.add(
                            egui::Label::new(lyric_layout(
                                ui,
                                text,
                                lyric_font(weight, size),
                                app.palette.text,
                                settings.display.letter_spacing,
                                egui::Align::Min,
                                ui.available_width(),
                            ))
                            .wrap(),
                        );
                    }
                });
        });
    });
}

fn lyric_rows<'a>(
    app: &'a App,
    lyrics: &'a crate::lyrics::Lyrics,
    index: usize,
    base_size: f32,
    color: Color32,
) -> Vec<(&'a str, egui::FontId, Color32)> {
    let lyric = &lyrics.lines[index];
    let text = if lyric.text.is_empty() && lyrics.synced {
        "♪"
    } else {
        &lyric.text
    };
    let originals: Vec<_> = text.split('\n').collect();
    let styled_rows = if lyrics.source == crate::lyrics::Source::Alsong && originals.len() == 1 {
        lyrics
            .lines
            .iter()
            .enumerate()
            .map(|(at, line)| (at.abs_diff(index), line.text.split('\n').count()))
            .filter(|(_, count)| matches!(count, 2 | 3))
            .min_by_key(|(distance, _)| *distance)
            .map_or(1, |(_, count)| count)
    } else {
        originals.len()
    };
    let settings = &app.settings.ivlyrics;
    let display = settings.display.validated();
    let mut rows = Vec::new();
    for (index, text) in originals.iter().enumerate() {
        let size = settings.line_size(lyrics.source, styled_rows, index, base_size);
        let weight = lyric_weight(settings, lyrics.source, styled_rows, index, index > 0);
        rows.push((*text, lyric_font(weight, size), color));
    }
    if settings.ai_enabled
        && let Loadable::Loaded(lines) = &app.ivlyrics.translated
        && let Some(extra) = lines.get(index)
    {
        for (enabled, text, scale, alpha) in [
            (
                settings.show_pronunciation,
                extra.pronunciation.as_str(),
                display.pronunciation_scale,
                0.85,
            ),
            (
                settings.show_translation,
                extra.translation.as_str(),
                display.translation_scale,
                0.9,
            ),
        ] {
            if enabled && !text.is_empty() {
                let size =
                    settings.line_size(crate::lyrics::Source::Unknown, 1, 0, base_size * scale);
                rows.push((
                    text,
                    lyric_font(crate::ivlyrics::FontWeight::Regular, size),
                    color.gamma_multiply(alpha),
                ));
            }
        }
    }
    rows
}

pub(super) fn panel_scale(app: &App, viewport: egui::Vec2) -> Option<f32> {
    app.settings.ivlyrics.display.panel_auto_fit.then(|| {
        // The base scale follows the viewport; individual near-fitting lines can shrink further.
        (viewport.x / 520.0)
            .min(viewport.y / 680.0)
            .clamp(0.45, 1.0)
    })
}

#[derive(Default)]
pub(super) struct LineLayout {
    pub fit: Option<f32>,
    pub clear_y: Option<egui::Rangef>,
}

/// Keep each timestamp clickable, with the same fit scale across the song.
pub(super) fn line(
    app: &App,
    ui: &mut egui::Ui,
    lyrics: &crate::lyrics::Lyrics,
    index: usize,
    font: egui::FontId,
    color: Color32,
    layout: LineLayout,
) -> egui::Response {
    let sense = if lyrics.synced {
        Sense::click()
    } else {
        Sense::hover()
    };
    let display = app.settings.ivlyrics.display.validated();
    let rows = lyric_rows(app, lyrics, index, font.size, color);
    let width = ui.available_width().max(1.0);
    let base_scale = layout.fit.unwrap_or(1.0);
    let mut shrink = 1.0;
    if display.panel_auto_fit {
        let widest = rows
            .iter()
            .map(|(text, font, color)| {
                let mut font = font.clone();
                font.size *= base_scale;
                lyric_layout(
                    ui,
                    text,
                    font,
                    *color,
                    display.letter_spacing * base_scale,
                    egui::Align::Min,
                    f32::INFINITY,
                )
                .size()
                .x
            })
            .fold(0.0, f32::max);
        let fit = (width - 2.0).max(1.0) / widest.max(1.0);
        // Keep the whole timestamp's row ratios; long passages still wrap at their normal size.
        if widest > width && (0.8..1.0).contains(&fit) {
            shrink = fit;
        }
    }
    let scale = base_scale * shrink;
    let mut response: Option<egui::Response> = None;
    let original_rows = lyrics.lines[index].text.split('\n').count();
    let normal_gap = ui.spacing().item_spacing.y;
    let fade_top = !(index == 0
        && app.mini_player.is_none()
        && app.lyrics_fullscreen.is_none()
        && ui.cursor().min.y >= ui.clip_rect().top());
    for (row_index, (text, mut font, color)) in rows.into_iter().enumerate() {
        ui.spacing_mut().item_spacing.y = app
            .settings
            .ivlyrics
            .row_spacing(lyrics.source, original_rows, row_index)
            .map_or(normal_gap * shrink, |gap| gap * scale);
        font.size *= scale;
        let galley = lyric_layout(
            ui,
            text,
            font,
            color,
            display.letter_spacing * scale,
            alignment(app),
            width,
        );
        let row = if app.believed_playing() {
            let (pos, galley, row) = egui::Label::new(galley)
                .selectable(false)
                .sense(sense)
                .layout_in_ui(ui);
            paint_faded_lyric(ui, pos, galley, color, fade_top, layout.clear_y);
            row
        } else {
            ui.add(egui::Label::new(galley).selectable(true))
        };
        row.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), text)
        });
        response = Some(match response.take() {
            Some(previous) => previous.union(row),
            None => row,
        });
    }
    ui.spacing_mut().item_spacing.y = normal_gap;
    response.expect("split always yields one row")
}

fn edge_alpha(y: f32, viewport: egui::Rect, clear: egui::Rangef, fade_top: bool) -> f32 {
    let bottom = (viewport.bottom() - y) / (viewport.bottom() - clear.max).max(1.0);
    let t = if fade_top {
        ((y - viewport.top()) / (clear.min - viewport.top()).max(1.0)).min(bottom)
    } else {
        bottom
    }
    .clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn paint_faded_lyric(
    ui: &egui::Ui,
    pos: egui::Pos2,
    galley: std::sync::Arc<egui::Galley>,
    color: Color32,
    fade_top: bool,
    clear_y: Option<egui::Rangef>,
) {
    let viewport = ui.clip_rect();
    let bounds = galley.rect.translate(pos.to_vec2());
    if !viewport.intersects(bounds) {
        return;
    }
    let fade = (viewport.height() * 0.14).clamp(32.0, 80.0);
    let clear = clear_y.unwrap_or(egui::Rangef::new(
        viewport.top() + fade,
        viewport.bottom() - fade,
    ));
    if (!fade_top || bounds.top() >= clear.min) && bounds.bottom() <= clear.max {
        ui.painter().galley(pos, galley, color);
        return;
    }
    // Fade the glyph vertices themselves; no clipped blur strips or background mask.
    let mut faded = (*galley).clone();
    for row in &mut faded.rows {
        let top = pos.y + row.pos.y;
        for vertex in &mut std::sync::Arc::make_mut(&mut row.row).visuals.mesh.vertices {
            let base = if vertex.color == Color32::PLACEHOLDER {
                color
            } else {
                vertex.color
            };
            vertex.color =
                base.gamma_multiply(edge_alpha(top + vertex.pos.y, viewport, clear, fade_top));
        }
    }
    ui.painter().galley(pos, std::sync::Arc::new(faded), color);
}

pub fn dialogs(app: &mut App, ctx: &egui::Context) {
    if app.ivlyrics.settings_open {
        settings(app, ctx);
    }
    if app.ivlyrics.candidates_open {
        candidates(app, ctx);
    }
}

fn settings(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let mut open = true;
    let mut settings = app
        .ivlyrics
        .settings_draft
        .clone()
        .unwrap_or_else(|| app.settings.ivlyrics.clone());
    settings.display = settings.display.validated();
    let original = settings.clone();
    let width = (ctx.content_rect().width() - 64.0).clamp(280.0, 720.0);
    let height = (ctx.content_rect().height() - 140.0).clamp(280.0, 780.0);
    let palette = app.palette;
    let art = app
        .now_playing()
        .and_then(|now| now.art_small.or(now.art_url));
    let tint = if settings.display.settings_gradient {
        app.tint_for(art.as_deref())
    } else {
        None
    };
    if app.mini_player.is_some() && !ctx.embed_viewports() {
        let parent = ctx
            .input(|input| input.viewport().outer_rect)
            .unwrap_or(ctx.content_rect());
        let work = crate::window::monitor_work_area(parent.center(), ctx.pixels_per_point())
            .unwrap_or_else(|| {
                egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    ctx.input(|input| input.viewport().monitor_size)
                        .unwrap_or(egui::vec2(1280.0, 800.0)),
                )
            });
        let rect = settings_window_rect(parent, work);
        let level = if app.mini_player.is_some_and(|mini| mini.always_on_top) {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("ivlyrics-settings-window"),
            egui::ViewportBuilder::default()
                .with_title(gettext(locale, "ivLyrics settings"))
                .with_decorations(!cfg!(windows))
                .with_position(rect.min)
                .with_inner_size(rect.size())
                .with_min_inner_size(egui::vec2(480.0, 360.0))
                .with_window_level(level),
            |ui, _| {
                open = !ui.input(|input| {
                    input.viewport().close_requested() || input.key_pressed(egui::Key::Escape)
                });
                crate::theme::apply_local(ui, &palette);
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(palette.window).inner_margin(24))
                    .show(ui, |ui| {
                        super::titlebar_drag(
                            ui,
                            egui::Rect::from_min_size(
                                ui.max_rect().min,
                                egui::vec2(ui.available_width(), 40.0),
                            ),
                        );
                        settings_contents(
                            app,
                            ui,
                            &mut settings,
                            ui.available_height() + 48.0,
                            tint,
                        );
                    });
                super::window_resize(ui);
            },
        );
    } else {
        egui::Window::new(gettext(locale, "ivLyrics settings"))
            .id(egui::Id::new("ivlyrics-settings"))
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_width(width)
            .default_height(height)
            .default_pos(ctx.content_rect().center() - egui::vec2(width, height) * 0.5)
            .min_width(width.min(700.0))
            .max_width(width)
            .max_height(ctx.content_rect().height() - 48.0)
            .frame(
                egui::Frame::new()
                    .fill(palette.window)
                    .corner_radius(14)
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                settings_contents(app, ui, &mut settings, height, tint)
            });
    }
    if settings != original {
        app.actions
            .push(Action::IvLyrics(IvAction::Draft(Box::new(settings))));
    }
    if !open {
        app.actions
            .push(Action::IvLyrics(IvAction::Settings(false)));
    }
}

fn settings_window_rect(parent: egui::Rect, work: egui::Rect) -> egui::Rect {
    let work = work.shrink(12.0);
    let right = work.right() - parent.right() - 12.0;
    let left = parent.left() - work.left() - 12.0;
    let on_right = right >= 720.0 || right >= left;
    let width = right.max(left).clamp(480.0, 720.0).min(work.width());
    let height = 740.0_f32.min((work.height() - 32.0).max(360.0));
    let x = if on_right {
        parent.right() + 12.0
    } else {
        parent.left() - width - 12.0
    };
    let pos = egui::pos2(
        x.clamp(work.left(), work.right() - width),
        parent.top().clamp(
            work.top() + 32.0,
            (work.bottom() - height).max(work.top() + 32.0),
        ),
    );
    egui::Rect::from_min_size(pos, egui::vec2(width, height))
}

fn settings_contents(
    app: &mut App,
    ui: &mut egui::Ui,
    settings: &mut crate::ivlyrics::Settings,
    height: f32,
    tint: Option<Color32>,
) {
    let palette = app.palette;
    let locale = app.locale;
    if let Some(tint) = tint {
        let rect =
            egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.available_width(), height));
        super::widgets::paint_vertical_gradient(
            ui,
            rect,
            super::blend(palette.window, tint, 0.28),
            palette.window,
        );
    }
    ui.horizontal(|ui| {
        crate::theme::text(
            ui,
            gettext(locale, "ivLyrics settings"),
            crate::theme::bold(24.0),
            palette.text,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = crate::theme::icon_button(
                ui,
                crate::theme::Icon::X,
                20.0,
                palette.secondary,
                palette.text,
                &gettext(locale, "Close"),
            )
            .clicked();
            if close {
                app.actions
                    .push(Action::IvLyrics(IvAction::Settings(false)));
            }
        });
    });
    ui.add_space(18.0);
    egui::ScrollArea::vertical()
        .id_salt("ivlyrics-settings-page")
        .max_height((ui.available_height() - 52.0).min(height - 126.0).max(80.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            settings_sections(app, ui, settings);
        });
    settings_footer(app, ui, settings);
}

pub(super) fn settings_page(app: &mut App, ui: &mut egui::Ui) {
    let mut settings = app
        .ivlyrics
        .settings_draft
        .clone()
        .unwrap_or_else(|| app.settings.ivlyrics.clone());
    settings.display = settings.display.validated();
    let original = settings.clone();
    crate::theme::text(
        ui,
        gettext(app.locale, "ivLyrics settings"),
        crate::theme::bold(24.0),
        app.palette.text,
    );
    ui.add_space(12.0);
    settings_sections(app, ui, &mut settings);
    settings_footer(app, ui, &mut settings);
    if settings != original {
        app.actions
            .push(Action::IvLyrics(IvAction::Draft(Box::new(settings))));
    }
}

fn settings_sections(app: &mut App, ui: &mut egui::Ui, settings: &mut crate::ivlyrics::Settings) {
    let palette = app.palette;
    let locale = app.locale;
    super::settings::section(ui, &palette, &gettext(locale, "Lyrics"), |ui| {
        provider_settings(app, ui, settings);
    });
    super::settings::section(
        ui,
        &palette,
        &gettext(locale, "Translation and pronunciation"),
        |ui| {
            translation_settings(app, ui, settings);
        },
    );
    super::settings::section(ui, &palette, &gettext(locale, "Appearance"), |ui| {
        appearance_settings(app, ui, settings);
    });
    super::settings::section(ui, &palette, &gettext(locale, "Full screen lyrics"), |ui| {
        fullscreen_settings(app, ui, settings);
    });
}

fn settings_footer(app: &mut App, ui: &mut egui::Ui, settings: &mut crate::ivlyrics::Settings) {
    let palette = app.palette;
    let locale = app.locale;
    ui.separator();
    ui.horizontal(|ui| {
        if crate::theme::pill_button(ui, &palette, &gettext(locale, "Reset"), false).clicked() {
            settings.display = Default::default();
            settings.font_scale = 1.0;
            let defaults = crate::ivlyrics::Settings::default();
            settings.two_row_sizes = defaults.two_row_sizes;
            settings.three_row_sizes = defaults.three_row_sizes;
            settings.two_row_spacing = defaults.two_row_spacing;
            settings.three_row_spacing = defaults.three_row_spacing;
            settings.font_weight = defaults.font_weight;
            settings.two_row_weights = defaults.two_row_weights;
            settings.three_row_weights = defaults.three_row_weights;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::theme::pill_button(ui, &palette, &gettext(locale, "Apply settings"), true)
                .clicked()
            {
                app.actions
                    .push(Action::IvLyrics(IvAction::Configure(Box::new(
                        settings.clone(),
                    ))));
            }
        });
    });
}

fn translation_settings(
    app: &mut App,
    ui: &mut egui::Ui,
    settings: &mut crate::ivlyrics::Settings,
) {
    let locale = app.locale;
    let old_provider = settings.translator.provider;
    toggle_setting(app, ui, &mut settings.ai_enabled, &gettext(locale, "AI ON"));
    ui.label(gettext(locale, "AI is off by default. Enabling it sends the displayed lyrics to your chosen provider and may use paid API quota."));
    egui::ComboBox::from_id_salt("ivlyrics-provider")
        .selected_text(settings.translator.provider.name())
        .show_ui(ui, |ui| {
            for provider in crate::ivlyrics::translate::Provider::ALL {
                ui.selectable_value(&mut settings.translator.provider, provider, provider.name());
            }
        });
    if settings.translator.provider != old_provider {
        settings.translator.model.clear();
    }
    let label = ui.label(gettext(locale, "Model ID"));
    ui.add(
        egui::TextEdit::singleline(&mut settings.translator.model).hint_text(gettext(
            locale,
            "Enter a chat model available to your API key",
        )),
    )
    .labelled_by(label.id);
    let label = ui.label(gettext(locale, "Translation language"));
    ui.text_edit_singleline(&mut settings.translator.language)
        .labelled_by(label.id);
    let label = ui.label(gettext(locale, "Pronunciation script"));
    ui.add(
        egui::TextEdit::singleline(&mut settings.translator.pronunciation)
            .hint_text("Korean Hangul / Latin / Japanese Hiragana"),
    )
    .labelled_by(label.id);
    let key_label = ui.label(gettext(locale, "API key"));
    let provider_pending =
        settings.translator.provider != app.settings.ivlyrics.translator.provider;
    let mut key = if !provider_pending {
        app.ivlyrics.api_key.clone()
    } else {
        String::new()
    };
    let busy = app.ivlyrics.key_request.is_some() || provider_pending;
    if provider_pending {
        ui.small(gettext(
            locale,
            "Apply settings before entering the new provider's key.",
        ));
    }
    if ui
        .add_enabled(
            !busy,
            egui::TextEdit::singleline(&mut key)
                .password(true)
                .desired_width(f32::INFINITY),
        )
        .labelled_by(key_label.id)
        .changed()
    {
        app.actions
            .push(Action::IvLyrics(IvAction::Key(Secret(key))));
    }
    ui.add_enabled_ui(!busy, |ui| {
        ui.horizontal_wrapped(|ui| {
            if crate::theme::pill_button(ui, &app.palette, &gettext(locale, "Save API key"), false)
                .clicked()
            {
                app.actions.push(Action::IvLyrics(IvAction::SaveKey));
            }
            if crate::theme::pill_button(
                ui,
                &app.palette,
                &gettext(locale, "Load saved key"),
                false,
            )
            .clicked()
            {
                app.actions.push(Action::IvLyrics(IvAction::LoadKey));
            }
        });
    });
    ui.small(gettext(
        locale,
        "Keys are stored in the system credential store. Save an empty key to remove it.",
    ));
    if let Some(status) = &app.ivlyrics.key_status {
        ui.small(status);
    }
    toggle_setting(
        app,
        ui,
        &mut settings.show_translation,
        &gettext(locale, "Show translation"),
    );
    toggle_setting(
        app,
        ui,
        &mut settings.show_pronunciation,
        &gettext(locale, "Show pronunciation"),
    );
}

fn appearance_settings(app: &App, ui: &mut egui::Ui, settings: &mut crate::ivlyrics::Settings) {
    let locale = app.locale;
    use crate::ivlyrics::Alignment;
    ui.label(gettext(locale, "Lyric alignment"));
    ui.horizontal(|ui| {
        for (value, label) in [
            (Alignment::Left, gettext(locale, "Left")),
            (Alignment::Center, gettext(locale, "Center")),
            (Alignment::Right, gettext(locale, "Right")),
        ] {
            if crate::theme::soft_button(
                ui,
                &app.palette,
                None,
                &label,
                settings.display.alignment == value,
            )
            .clicked()
            {
                settings.display.alignment = value;
            }
        }
    });
    toggle_setting(
        app,
        ui,
        &mut settings.display.panel_auto_fit,
        &gettext(locale, "Auto-size lyrics to available space"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.font_scale,
        0.5..=3.0,
        &gettext(locale, "Lyric text scale"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.letter_spacing,
        -2.0..=12.0,
        &gettext(locale, "Letter spacing (px)"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.line_spacing,
        0.0..=120.0,
        &gettext(locale, "Lyric group spacing (px)"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.inactive_opacity,
        0.15..=0.85,
        &gettext(locale, "Inactive lyric opacity"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.translation_scale,
        0.4..=1.2,
        &gettext(locale, "Translation text scale"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.pronunciation_scale,
        0.4..=1.2,
        &gettext(locale, "Pronunciation text scale"),
    );
    super::widgets::setting_row(
        ui,
        &app.palette,
        &gettext(locale, "Font weight"),
        "",
        |ui| {
            weight_picker(ui, locale, &mut settings.font_weight);
        },
    );
    ui.separator();
    row_type_settings(app, ui, settings, 2);
    row_type_settings(app, ui, settings, 3);

    ui.separator();
    toggle_setting(
        app,
        ui,
        &mut settings.display.album_art_background,
        &gettext(locale, "Album artwork background"),
    );
    toggle_setting(
        app,
        ui,
        &mut settings.display.settings_gradient,
        &gettext(locale, "Album colour gradient"),
    );
    slider_setting(
        app,
        ui,
        &mut settings.display.background_dim,
        0.0..=0.9,
        &gettext(locale, "Background dimming"),
    );
}

fn fullscreen_settings(app: &App, ui: &mut egui::Ui, settings: &mut crate::ivlyrics::Settings) {
    let locale = app.locale;
    let display = &mut settings.display;
    toggle_setting(
        app,
        ui,
        &mut display.two_columns,
        &gettext(locale, "Two-column layout"),
    );
    ui.add_enabled_ui(display.two_columns, |ui| {
        toggle_setting(
            app,
            ui,
            &mut display.reversed,
            &gettext(locale, "Reverse album and lyrics"),
        );
    });
    ui.separator();
    toggle_setting(
        app,
        ui,
        &mut display.show_album,
        &gettext(locale, "Show album artwork"),
    );
    ui.add_enabled_ui(display.show_album, |ui| {
        slider_setting(
            app,
            ui,
            &mut display.album_size,
            160.0..=600.0,
            &gettext(locale, "Album artwork size"),
        );
        slider_setting(
            app,
            ui,
            &mut display.album_radius,
            0.0..=64.0,
            &gettext(locale, "Album corner radius"),
        );
    });
    toggle_setting(
        app,
        ui,
        &mut display.show_info,
        &gettext(locale, "Show track information"),
    );
    ui.add_enabled_ui(display.show_info, |ui| {
        toggle_setting(
            app,
            ui,
            &mut display.show_album_name,
            &gettext(locale, "Show album name"),
        );
        slider_setting(
            app,
            ui,
            &mut display.title_size,
            18.0..=64.0,
            &gettext(locale, "Title text size"),
        );
        slider_setting(
            app,
            ui,
            &mut display.artist_size,
            12.0..=36.0,
            &gettext(locale, "Artist text size"),
        );
    });
    ui.separator();
    toggle_setting(
        app,
        ui,
        &mut display.show_controls,
        &gettext(locale, "Show playback controls"),
    );
    ui.add_enabled_ui(display.show_controls, |ui| {
        toggle_setting(
            app,
            ui,
            &mut display.controls_below_album,
            &gettext(locale, "Controls below album"),
        );
        toggle_setting(
            app,
            ui,
            &mut display.show_progress,
            &gettext(locale, "Show progress bar"),
        );
        toggle_setting(
            app,
            ui,
            &mut display.show_volume,
            &gettext(locale, "Show volume controls"),
        );
    });
}

fn candidates(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let mut open = true;
    let mut close_requested = false;
    let width = (ctx.content_rect().width() - 64.0).clamp(300.0, 940.0);
    let height = (ctx.content_rect().height() - 384.0).clamp(140.0, 500.0);
    egui::Window::new(gettext(locale, "ALSong candidates"))
        .id(egui::Id::new("ivlyrics-candidates"))
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .default_width(width)
        .default_pos(ctx.content_rect().center() - egui::vec2(width, height + 240.0) * 0.5)
        .max_width(width)
        .frame(
            egui::Frame::new()
                .fill(palette.window)
                .corner_radius(14)
                .inner_margin(24),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                crate::theme::text(
                    ui,
                    gettext(locale, "ALSong candidates"),
                    crate::theme::bold(24.0),
                    palette.text,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close_requested = crate::theme::icon_button(
                        ui,
                        crate::theme::Icon::X,
                        20.0,
                        palette.secondary,
                        palette.text,
                        &gettext(locale, "Close"),
                    )
                    .clicked();
                });
            });
            ui.add_space(16.0);
            if let Some(now) = app.now_playing() {
                ui.add(
                    egui::Label::new(egui::RichText::new(&now.title).heading()).selectable(true),
                );
                ui.add(egui::Label::new(&now.subtitle).selectable(true));
                let (mut title, mut artist) =
                    app.ivlyrics.search_terms.clone().unwrap_or_else(|| {
                        (
                            now.title.clone(),
                            now.artists
                                .first()
                                .map(|artist| artist.name.clone())
                                .unwrap_or_default(),
                        )
                    });
                ui.add_space(8.0);
                let field_width = ((ui.available_width() - 100.0) / 2.0).max(70.0);
                let mut changed = false;
                let mut search = false;
                ui.horizontal(|ui| {
                    let title_input = ui
                        .vertical(|ui| {
                            ui.set_width(field_width);
                            super::widgets::labeled_field(
                                ui,
                                &palette,
                                locale,
                                &gettext(locale, "Song title"),
                                &mut title,
                                "",
                                false,
                            )
                        })
                        .inner;
                    let artist_input = ui
                        .vertical(|ui| {
                            ui.set_width(field_width);
                            super::widgets::labeled_field(
                                ui,
                                &palette,
                                locale,
                                &gettext(locale, "Artist"),
                                &mut artist,
                                "",
                                false,
                            )
                        })
                        .inner;
                    changed = title_input.changed() || artist_input.changed();
                    let enabled =
                        !app.offline && (!title.trim().is_empty() || !artist.trim().is_empty());
                    let enter = (title_input.lost_focus() || artist_input.lost_focus())
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    search = ui
                        .add_enabled_ui(enabled, |ui| {
                            crate::theme::pill_button(
                                ui,
                                &palette,
                                &gettext(locale, "Search"),
                                true,
                            )
                        })
                        .inner
                        .clicked()
                        || (enabled && enter);
                });
                if changed {
                    app.actions
                        .push(Action::IvLyrics(IvAction::SearchTerms { title, artist }));
                }
                if search {
                    app.actions.push(Action::IvLyrics(IvAction::Search(1)));
                }
            }
            ui.add_space(8.0);
            ui.label(gettext(
                locale,
                "Preview a candidate before using it. Your choice is remembered for this song.",
            ));
            ui.add_space(10.0);
            if crate::theme::pill_button(
                ui,
                &palette,
                &gettext(locale, "Restore automatic selection"),
                false,
            )
            .clicked()
            {
                app.actions.push(Action::IvLyrics(IvAction::Select(None)));
            }
            ui.add_space(16.0);
            let divider = ui.available_rect_before_wrap().center().x;
            let top = ui.cursor().top();
            ui.painter().line_segment(
                [egui::pos2(divider, top), egui::pos2(divider, top + height)],
                egui::Stroke::new(1.0, palette.outline),
            );
            ui.spacing_mut().item_spacing.x = 24.0;
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                ui.set_min_height(height);
                ui.strong(gettext(locale, "Search results"));
                ui.add_space(10.0);
                match &app.ivlyrics.candidates {
                    Loadable::NotLoaded | Loadable::Failed(_) => {
                        if let Loadable::Failed(error) = &app.ivlyrics.candidates {
                            ui.label(error);
                        }
                    }
                    Loadable::Loading => {
                        ui.spinner();
                    }
                    Loadable::Loaded(candidates) => {
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_enabled_ui(candidates.page > 1, |ui| {
                                    crate::theme::pill_button(
                                        ui,
                                        &palette,
                                        &gettext(locale, "Previous page"),
                                        false,
                                    )
                                })
                                .inner
                                .clicked()
                            {
                                app.actions
                                    .push(Action::IvLyrics(IvAction::Search(candidates.page - 1)));
                            }
                            ui.label(candidates.page.to_string());
                            if ui
                                .add_enabled_ui(
                                    candidates.has_more && candidates.page < 100,
                                    |ui| {
                                        crate::theme::pill_button(
                                            ui,
                                            &palette,
                                            &gettext(locale, "Next page"),
                                            false,
                                        )
                                    },
                                )
                                .inner
                                .clicked()
                            {
                                app.actions
                                    .push(Action::IvLyrics(IvAction::Search(candidates.page + 1)));
                            }
                        });
                        ui.add_space(10.0);
                        egui::ScrollArea::vertical()
                            .id_salt("alsong-candidate-list")
                            .max_height(height - 90.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if candidates.entries.is_empty() {
                                    ui.label(gettext(
                                        locale,
                                        "No matching candidates on this page.",
                                    ));
                                }
                                for candidate in &candidates.entries {
                                    let selected =
                                        app.ivlyrics.preview_candidate.as_ref() == Some(candidate);
                                    let label = format!(
                                        "{} · {}\n{} · #{}",
                                        candidate.title,
                                        candidate.artist,
                                        candidate.album,
                                        candidate.id
                                    );
                                    let button = egui::Button::new(label)
                                        .selected(selected)
                                        .wrap()
                                        .corner_radius(14)
                                        .min_size(egui::vec2(ui.available_width(), 64.0));
                                    if ui.add(button).clicked() {
                                        app.actions.push(Action::IvLyrics(IvAction::Preview(
                                            candidate.clone(),
                                        )));
                                    }
                                    ui.add_space(6.0);
                                }
                            });
                    }
                }
                let ui = &mut columns[1];
                ui.strong(gettext(locale, "Selected lyrics"));
                ui.add_space(10.0);
                match &app.ivlyrics.preview {
                    Loadable::Loading => {
                        ui.spinner();
                    }
                    Loadable::Failed(error) => {
                        ui.label(error);
                    }
                    Loadable::Loaded(lyrics) => {
                        if crate::theme::pill_button(
                            ui,
                            &palette,
                            &gettext(locale, "Use these lyrics"),
                            true,
                        )
                        .clicked()
                        {
                            app.actions.push(Action::IvLyrics(IvAction::Select(
                                app.ivlyrics.preview_candidate.clone(),
                            )));
                        }
                        ui.add_space(10.0);
                        egui::ScrollArea::vertical()
                            .id_salt("alsong-preview")
                            .max_height(height - 90.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for line in &lyrics.lines {
                                    if let Some(at) = line.at_ms {
                                        ui.weak(format!(
                                            "{:02}:{:02}",
                                            at / 60_000,
                                            at / 1000 % 60
                                        ));
                                    }
                                    ui.add(egui::Label::new(&line.text).wrap());
                                    ui.add_space(12.0);
                                }
                            });
                    }
                    _ => {
                        ui.label(gettext(locale, "Select a candidate to preview its lyrics."));
                    }
                }
            });
        });
    if !open || close_requested {
        app.actions
            .push(Action::IvLyrics(IvAction::Candidates(false)));
    }
}

fn provider_settings(app: &mut App, ui: &mut egui::Ui, settings: &mut crate::ivlyrics::Settings) {
    let locale = app.locale;
    ui.label(gettext(locale, "Saved song settings"));
    ui.small(gettext(
        locale,
        "The provider, ALSong choice and timing offset are saved automatically for each song.",
    ));
    if let Some(now) = app.now_playing().filter(|now| !now.is_episode) {
        let selected = app.settings.ivlyrics.provider(&now.uri);
        ui.label(format!(
            "{} · {}",
            gettext(locale, "Current song provider"),
            now.title
        ));
        ui.horizontal_wrapped(|ui| {
            for source in [
                None,
                Some(crate::lyrics::Source::Alsong),
                Some(crate::lyrics::Source::Spotify),
                Some(crate::lyrics::Source::Lrclib),
            ] {
                let label = source
                    .map(|source| source.name().to_owned())
                    .unwrap_or_else(|| gettext(locale, "Automatic").into_owned());
                if crate::theme::soft_button(ui, &app.palette, None, &label, selected == source)
                    .clicked()
                {
                    app.actions
                        .push(Action::IvLyrics(IvAction::Provider(source)));
                }
            }
        });
    }
    ui.add_space(8.0);
    if ui
        .add_enabled_ui(app.ivlyrics.cache_request.is_none(), |ui| {
            crate::theme::pill_button(
                ui,
                &app.palette,
                &gettext(locale, "Clear lyrics and translation cache"),
                false,
            )
        })
        .inner
        .clicked()
    {
        app.actions.push(Action::IvLyrics(IvAction::ClearCache));
    }
    ui.small(gettext(locale, "Keeps song settings and current lyrics. Lyrics and translations are fetched again when needed."));
    if let Some(status) = &app.ivlyrics.cache_status {
        ui.label(status);
    }
    ui.separator();
    ui.label(gettext(locale, "Lyric provider order"));
    ui.small(gettext(
        locale,
        "Automatic selection tries providers from top to bottom.",
    ));
    let mut order = settings.ordered_providers();
    let mut swap = None;
    for (index, source) in order.iter().enumerate() {
        egui::Frame::new()
            .fill(app.palette.surface)
            .stroke(egui::Stroke::new(1.0, app.palette.outline))
            .corner_radius(12)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_min_width((ui.available_width() - 2.0).max(0.0));
                ui.horizontal(|ui| {
                    ui.label(format!("{}   {}", index + 1, source.name()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled_ui(index + 1 < order.len(), |ui| {
                                let response =
                                    crate::theme::pill_button(ui, &app.palette, "↓", false);
                                let label =
                                    format!("{}: {}", source.name(), gettext(locale, "Move down"));
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        ui.is_enabled(),
                                        &label,
                                    )
                                });
                                response.on_hover_text(label)
                            })
                            .inner
                            .clicked()
                        {
                            swap = Some((index, index + 1));
                        }
                        if ui
                            .add_enabled_ui(index > 0, |ui| {
                                let response =
                                    crate::theme::pill_button(ui, &app.palette, "↑", false);
                                let label =
                                    format!("{}: {}", source.name(), gettext(locale, "Move up"));
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        ui.is_enabled(),
                                        &label,
                                    )
                                });
                                response.on_hover_text(label)
                            })
                            .inner
                            .clicked()
                        {
                            swap = Some((index, index - 1));
                        }
                    });
                });
            });
        ui.add_space(6.0);
    }
    if let Some((from, to)) = swap {
        order.swap(from, to);
        settings.provider_order = order;
        settings.selected_provider = None;
    }
    ui.label(gettext(locale, "Default provider for other songs"));
    ui.horizontal_wrapped(|ui| {
        if crate::theme::soft_button(
            ui,
            &app.palette,
            None,
            &gettext(locale, "Automatic"),
            settings.selected_provider.is_none(),
        )
        .clicked()
        {
            settings.selected_provider = None;
        }
        for source in [
            crate::lyrics::Source::Alsong,
            crate::lyrics::Source::Spotify,
            crate::lyrics::Source::Lrclib,
        ] {
            if crate::theme::soft_button(
                ui,
                &app.palette,
                None,
                source.name(),
                settings.selected_provider == Some(source),
            )
            .clicked()
            {
                settings.selected_provider = Some(source);
            }
        }
    });
    ui.add_space(8.0);
    if ui
        .add_enabled_ui(!app.offline, |ui| {
            crate::theme::pill_button(
                ui,
                &app.palette,
                &gettext(locale, "ALSong candidates"),
                false,
            )
        })
        .inner
        .clicked()
    {
        app.actions
            .push(Action::IvLyrics(IvAction::Candidates(true)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_window_uses_the_free_side_and_stays_in_the_work_area() {
        let work = egui::Rect::from_min_max(egui::pos2(-1920.0, 0.0), egui::pos2(0.0, 1040.0));
        for x in [-1900.0, -400.0] {
            let parent = egui::Rect::from_min_size(egui::pos2(x, 200.0), egui::vec2(380.0, 600.0));
            let settings = settings_window_rect(parent, work);
            assert!(work.contains_rect(settings));
            assert!(!settings.intersects(parent));
            assert!(settings.width() >= 600.0);
            if x < -1000.0 {
                assert!(settings.left() > parent.right());
            } else {
                assert!(settings.right() < parent.left());
            }
        }
    }

    #[test]
    fn the_old_gradient_switch_does_not_disable_the_restored_artwork() {
        let display: crate::ivlyrics::Display =
            serde_json::from_str(r#"{"background_art":false}"#).unwrap();
        assert!(!display.settings_gradient);
        assert!(display.album_art_background);
        let saved = serde_json::to_string(&display).unwrap();
        assert!(saved.contains("settings_gradient"));
        assert!(!saved.contains("background_art"));
    }

    #[test]
    fn letter_spacing_preserves_text_and_expands_the_shared_layout() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let text = "가사 Lyrics 歌詞";
            let normal = lyric_layout(
                ui,
                text,
                crate::theme::regular(24.0),
                Color32::WHITE,
                0.0,
                egui::Align::Min,
                ui.available_width(),
            );
            let spaced = lyric_layout(
                ui,
                text,
                crate::theme::regular(24.0),
                Color32::WHITE,
                4.0,
                egui::Align::Min,
                ui.available_width(),
            );
            assert_eq!(spaced.job.text, text);
            assert!(spaced.size().x > normal.size().x);
            assert!(
                spaced
                    .job
                    .sections
                    .iter()
                    .all(|section| section.format.extra_letter_spacing == 4.0)
            );
        });
        output.textures_delta.clear();
        let old: crate::ivlyrics::Display = serde_json::from_str("{}").unwrap();
        assert_eq!(old.letter_spacing, -0.5);
        let invalid = crate::ivlyrics::Display {
            letter_spacing: 100.0,
            ..old
        };
        assert_eq!(invalid.validated().letter_spacing, 12.0);
    }

    #[test]
    fn lyric_edges_fade_symmetrically_without_dimming_the_center() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 500.0));
        assert_eq!(
            edge_alpha(0.0, viewport, egui::Rangef::new(80.0, 420.0), true),
            0.0
        );
        assert_eq!(
            edge_alpha(500.0, viewport, egui::Rangef::new(80.0, 420.0), true),
            0.0
        );
        assert_eq!(
            edge_alpha(250.0, viewport, egui::Rangef::new(80.0, 420.0), true),
            1.0
        );
        assert_eq!(
            edge_alpha(0.0, viewport, egui::Rangef::new(80.0, 420.0), false),
            1.0
        );
        assert_eq!(
            edge_alpha(500.0, viewport, egui::Rangef::new(80.0, 420.0), false),
            0.0
        );
        for y in [5.0, 20.0, 40.0, 60.0] {
            assert_eq!(
                edge_alpha(y, viewport, egui::Rangef::new(80.0, 420.0), true),
                edge_alpha(500.0 - y, viewport, egui::Rangef::new(80.0, 420.0), true)
            );
            assert!(
                edge_alpha(y, viewport, egui::Rangef::new(80.0, 420.0), true)
                    < edge_alpha(y + 10.0, viewport, egui::Rangef::new(80.0, 420.0), true)
            );
        }
    }

    #[test]
    fn lyric_fade_starts_at_the_album_and_control_bounds() {
        let viewport =
            egui::Rect::from_min_max(egui::pos2(900.0, 80.0), egui::pos2(1800.0, 1000.0));
        let clear = egui::Rangef::new(160.0, 920.0);
        for y in [160.0, 500.0, 920.0] {
            assert_eq!(edge_alpha(y, viewport, clear, true), 1.0);
        }
        for y in [120.0, 960.0] {
            assert_eq!(edge_alpha(y, viewport, clear, true), 0.5);
        }
        for y in [80.0, 1000.0] {
            assert_eq!(edge_alpha(y, viewport, clear, true), 0.0);
        }
        // Unequal margins keep their own gradient widths after resizing.
        let clear = egui::Rangef::new(240.0, 960.0);
        assert_eq!(edge_alpha(160.0, viewport, clear, true), 0.5);
        assert_eq!(edge_alpha(980.0, viewport, clear, true), 0.5);
    }

    #[test]
    fn preview_and_lyrics_share_each_rows_selected_size_and_weight() {
        use crate::ivlyrics::FontWeight;
        let mut settings = crate::ivlyrics::Settings::default();
        assert_eq!(settings.two_row_sizes, [36.0, 38.0]);
        assert_eq!(settings.three_row_sizes, [36.0, 20.0, 38.0]);
        assert_eq!(
            settings.two_row_weights,
            [FontWeight::Medium, FontWeight::Semibold]
        );
        assert_eq!(
            settings.three_row_weights,
            [
                FontWeight::Medium,
                FontWeight::Semibold,
                FontWeight::Semibold
            ]
        );
        assert_eq!(settings.display.background_dim, 0.05);
        settings.three_row_weights = [FontWeight::Regular, FontWeight::Medium, FontWeight::Bold];
        for row in 0..3 {
            assert_eq!(
                lyric_weight(&settings, crate::lyrics::Source::Alsong, 3, row, row > 0),
                settings.three_row_weights[row]
            );
            assert_eq!(
                settings.line_size(crate::lyrics::Source::Alsong, 3, row, 30.0),
                settings.three_row_sizes[row]
            );
        }
        let saved = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<crate::ivlyrics::Settings>(&saved).unwrap(),
            settings
        );
    }
    #[test]
    fn near_fitting_lyrics_shrink_together_without_squeezing_long_passages() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let root =
            std::env::temp_dir().join(format!("spotifast-adaptive-lyrics-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            crate::settings::Settings::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
            },
        );
        let rows = [
            "Another way to feel what you didn't want yourself to know",
            "발음",
            "알고 싶지 않았던 것을 느끼는 다른 방법",
        ];
        let lyrics = crate::lyrics::Lyrics {
            source: crate::lyrics::Source::Alsong,
            lines: vec![crate::lyrics::Line {
                at_ms: Some(0),
                text: rows.join("\n"),
            }],
            ..Default::default()
        };
        let draw = |app: &App, width, scale| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width + 20.0, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.set_width(width);
                    line(
                        app,
                        ui,
                        &lyrics,
                        0,
                        crate::theme::regular(32.0),
                        egui::Color32::WHITE,
                        LineLayout {
                            fit: Some(scale),
                            ..Default::default()
                        },
                    );
                },
            );
            output.textures_delta.clear();
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if rows.contains(&text.galley.job.text.as_str()) => {
                        Some((
                            text.galley.job.sections[0].format.font_id.size,
                            text.galley.rows.len(),
                            text.galley.size().x,
                        ))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for scale in [0.6, 1.0] {
            app.settings.ivlyrics.display.panel_auto_fit = false;
            let baseline = draw(&app, 10_000.0, scale);
            assert_eq!(baseline.len(), 3);
            let widest = baseline.iter().map(|row| row.2).fold(0.0, f32::max);
            for enabled in [false, true] {
                app.settings.ivlyrics.display.panel_auto_fit = enabled;
                for fraction in [1.1, 0.93, 0.4] {
                    let shown = draw(&app, widest * fraction, scale);
                    assert_eq!(
                        shown.len(),
                        3,
                        "explicit original/pronunciation/translation rows stay separate"
                    );
                    if enabled && fraction == 0.93 {
                        assert!((0.8..1.0).contains(&(shown[0].0 / baseline[0].0)));
                        assert!(
                            shown.iter().all(|row| row.1 == 1),
                            "a short trailing wrap fits on one line"
                        );
                    } else {
                        assert_eq!(shown[0].0, baseline[0].0);
                        assert_eq!(shown.iter().any(|row| row.1 > 1), fraction < 1.0);
                    }
                    for index in 1..3 {
                        assert!(
                            (shown[index].0 / shown[0].0 - baseline[index].0 / baseline[0].0).abs()
                                < 0.001
                        );
                    }
                }
            }
        }
        app.backend.shutdown();
    }

    #[test]
    fn explicit_row_gaps_scale_with_the_lyrics_at_each_panel_width() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let root = std::env::temp_dir().join(format!("spotifast-row-gaps-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            crate::settings::Settings::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
            },
        );
        let full = panel_scale(&app, egui::vec2(520.0, 680.0)).unwrap();
        let narrow = panel_scale(&app, egui::vec2(360.0, 680.0)).unwrap();
        let short = panel_scale(&app, egui::vec2(520.0, 340.0)).unwrap();
        assert_eq!(full, 1.0);
        assert!(short < narrow && narrow < full);
        assert_eq!(panel_scale(&app, egui::vec2(1040.0, 1360.0)), Some(1.0));
        assert_eq!(panel_scale(&app, egui::Vec2::ZERO), Some(0.45));
        app.settings.ivlyrics.display.panel_auto_fit = false;
        assert_eq!(panel_scale(&app, egui::vec2(360.0, 500.0)), None);
        app.settings.ivlyrics.display.panel_auto_fit = true;
        let mixed = crate::lyrics::Lyrics {
            source: crate::lyrics::Source::Alsong,
            lines: vec![
                crate::lyrics::Line {
                    at_ms: Some(0),
                    text: "原文\n발음\n번역".into(),
                },
                crate::lyrics::Line {
                    at_ms: Some(1000),
                    text: "Distortion it’s Motion".into(),
                },
            ],
            ..Default::default()
        };
        app.settings.ivlyrics.three_row_sizes[0] = 48.0;
        assert_eq!(
            lyric_rows(&app, &mixed, 1, 22.0, Color32::WHITE)[0].1.size,
            48.0
        );
        app.settings.ivlyrics.three_row_sizes[0] = 38.0;
        app.settings.ivlyrics.two_row_spacing = 24.0;
        app.settings.ivlyrics.three_row_spacing = [12.0, 30.0];
        for count in [2, 3] {
            let lyrics = crate::lyrics::Lyrics {
                source: crate::lyrics::Source::Alsong,
                lines: vec![crate::lyrics::Line {
                    at_ms: Some(0),
                    text: [
                        "A quiet road beneath the moon",
                        "달 아래 고요한 길",
                        "먼 곳의 빛을 따라서",
                    ][..count]
                        .join("\n"),
                }],
                ..Default::default()
            };
            for width in [240.0, 700.0] {
                let mut scale = 1.0;
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    ui.set_width(width);
                    ui.add_space(100.0);
                    scale = panel_scale(&app, egui::vec2(width, 600.0)).unwrap();
                    line(
                        &app,
                        ui,
                        &lyrics,
                        0,
                        crate::theme::regular(38.0),
                        egui::Color32::WHITE,
                        LineLayout {
                            fit: Some(scale),
                            ..Default::default()
                        },
                    );
                });
                output.textures_delta.clear();
                let rects: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => {
                            Some(text.galley.rect.translate(text.pos.to_vec2()))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(rects.len(), count);
                for row in 0..count - 1 {
                    let expected = app
                        .settings
                        .ivlyrics
                        .row_spacing(crate::lyrics::Source::Alsong, count, row)
                        .unwrap()
                        * scale;
                    let actual = rects[row + 1].top() - rects[row].bottom();
                    assert!(
                        (actual - expected).abs() < 1.5,
                        "{width}: {actual} / {expected}"
                    );
                }
            }
        }
        app.backend.shutdown();
    }
}
