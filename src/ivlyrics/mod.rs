//! Native ivLyrics integration. Network work is dispatched by the backend.

pub mod alsong;
pub mod translate;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::lyrics::{Lyrics, Query};
use crate::model::Loadable;

#[derive(Clone)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Queue(bool),
    QueueVisible(bool),
    Settings(bool),
    SettingsSection(usize),
    Candidates(bool),
    Configure(Box<Settings>),
    Draft(Box<Settings>),
    Key(Secret),
    SaveKey,
    LoadKey,
    Search(u32),
    SearchTerms { title: String, artist: String },
    Preview(alsong::Candidate),
    Select(Option<alsong::Candidate>),
    Offset(i32),
    ToggleAi,
    RetryTranslation,
    Provider(Option<crate::lyrics::Source>),
    ClearCache,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub alsong_first: bool,
    pub provider_order: Vec<crate::lyrics::Source>,
    #[serde(default)]
    pub selected_provider: Option<crate::lyrics::Source>,
    pub ai_enabled: bool,
    pub translator: translate::Config,
    pub show_translation: bool,
    pub show_pronunciation: bool,
    pub font_scale: f32,
    pub display: Display,
    /// Zero inherits the normal lyric size. Only ALSong's explicit rows use these.
    pub two_row_sizes: [f32; 2],
    pub three_row_sizes: [f32; 3],
    pub two_row_spacing: f32,
    pub three_row_spacing: [f32; 2],
    pub font_weight: FontWeight,
    pub two_row_weights: [FontWeight; 2],
    pub three_row_weights: [FontWeight; 3],
    pub tracks: BTreeMap<String, TrackSettings>,
}

impl Settings {
    pub fn provider(&self, uri: &str) -> Option<crate::lyrics::Source> {
        self.tracks
            .get(uri)
            .and_then(|track| track.provider)
            .or(self.selected_provider)
            .filter(|source| *source != crate::lyrics::Source::Unknown)
    }

    pub fn providers(&self, uri: &str) -> Vec<crate::lyrics::Source> {
        if let Some(source) = self.provider(uri) {
            return vec![source];
        }
        self.ordered_providers()
    }

    pub fn ordered_providers(&self) -> Vec<crate::lyrics::Source> {
        use crate::lyrics::Source;
        let mut order = Vec::new();
        let defaults = if self.alsong_first {
            [Source::Alsong, Source::Spotify, Source::Lrclib]
        } else {
            [Source::Spotify, Source::Lrclib, Source::Alsong]
        };
        for source in self.provider_order.iter().copied().chain(defaults) {
            if source != Source::Unknown && !order.contains(&source) {
                order.push(source);
            }
        }
        order
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            alsong_first: true,
            provider_order: Vec::new(),
            selected_provider: Some(crate::lyrics::Source::Alsong),
            ai_enabled: false,
            translator: translate::Config::default(),
            show_translation: true,
            show_pronunciation: true,
            font_scale: 1.0,
            display: Display::default(),
            two_row_sizes: [36.0, 38.0],
            three_row_sizes: [36.0, 20.0, 38.0],
            two_row_spacing: 4.0,
            three_row_spacing: [3.0, 3.0],
            font_weight: FontWeight::Bold,
            two_row_weights: [FontWeight::Medium, FontWeight::Semibold],
            three_row_weights: [
                FontWeight::Medium,
                FontWeight::Semibold,
                FontWeight::Semibold,
            ],
            tracks: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Regular,
    Medium,
    Semibold,
    #[default]
    Bold,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Display {
    pub alignment: Alignment,
    pub two_columns: bool,
    pub reversed: bool,
    pub show_album: bool,
    pub show_info: bool,
    pub show_album_name: bool,
    pub album_size: f32,
    pub album_radius: f32,
    pub title_size: f32,
    pub artist_size: f32,
    #[serde(alias = "background_art")]
    pub settings_gradient: bool,
    pub album_art_background: bool,
    pub background_dim: f32,
    pub line_spacing: f32,
    pub letter_spacing: f32,
    pub panel_auto_fit: bool,
    pub inactive_opacity: f32,
    pub translation_scale: f32,
    pub pronunciation_scale: f32,
    pub controls_below_album: bool,
    pub show_controls: bool,
    pub show_progress: bool,
    pub show_volume: bool,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            alignment: Alignment::Center,
            two_columns: true,
            reversed: false,
            show_album: true,
            show_info: true,
            show_album_name: true,
            album_size: 400.0,
            album_radius: 12.0,
            title_size: 38.0,
            artist_size: 20.0,
            settings_gradient: false,
            album_art_background: true,
            background_dim: 0.05,
            line_spacing: 40.0,
            letter_spacing: -0.5,
            panel_auto_fit: true,
            inactive_opacity: 0.25,
            translation_scale: 0.72,
            pronunciation_scale: 0.62,
            controls_below_album: true,
            show_controls: true,
            show_progress: true,
            show_volume: true,
        }
    }
}

impl Display {
    pub fn validated(mut self) -> Self {
        let defaults = Self::default();
        for (value, default, min, max) in [
            (&mut self.album_size, defaults.album_size, 160.0, 600.0),
            (&mut self.album_radius, defaults.album_radius, 0.0, 64.0),
            (&mut self.title_size, defaults.title_size, 18.0, 64.0),
            (&mut self.artist_size, defaults.artist_size, 12.0, 36.0),
            (&mut self.background_dim, defaults.background_dim, 0.0, 0.9),
            (&mut self.line_spacing, defaults.line_spacing, 0.0, 120.0),
            (
                &mut self.letter_spacing,
                defaults.letter_spacing,
                -2.0,
                12.0,
            ),
            (
                &mut self.inactive_opacity,
                defaults.inactive_opacity,
                0.15,
                0.85,
            ),
            (
                &mut self.translation_scale,
                defaults.translation_scale,
                0.4,
                1.2,
            ),
            (
                &mut self.pronunciation_scale,
                defaults.pronunciation_scale,
                0.4,
                1.2,
            ),
        ] {
            *value = if value.is_finite() {
                value.clamp(min, max)
            } else {
                default
            };
        }
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackSettings {
    /// None inherits the default; Unknown explicitly selects automatic lookup.
    pub provider: Option<crate::lyrics::Source>,
    /// Positive means show each lyric later in the song.
    pub offset_ms: i32,
    pub alsong: Option<alsong::Candidate>,
}

impl Settings {
    pub fn offset(&self, uri: &str) -> i32 {
        self.tracks
            .get(uri)
            .map_or(0, |track| track.offset_ms)
            .clamp(-60_000, 60_000)
    }

    pub fn position(&self, uri: &str, position_ms: u32) -> u32 {
        (i64::from(position_ms) - i64::from(self.offset(uri))).clamp(0, i64::from(u32::MAX)) as u32
    }

    pub fn active_line(&self, uri: &str, lyrics: &Lyrics, position_ms: u32) -> Option<usize> {
        if i64::from(position_ms) < i64::from(self.offset(uri)) {
            return None;
        }
        lyrics.active_line(self.position(uri, position_ms))
    }

    pub fn seek_position(&self, uri: &str, at_ms: u32) -> u32 {
        (i64::from(at_ms) + i64::from(self.offset(uri))).clamp(0, i64::from(u32::MAX)) as u32
    }

    pub fn row_spacing(
        &self,
        source: crate::lyrics::Source,
        rows: usize,
        after_row: usize,
    ) -> Option<f32> {
        if source != crate::lyrics::Source::Alsong {
            return None;
        }
        let gap = match (rows, after_row) {
            (2, 0) => self.two_row_spacing,
            (3, 0..=1) => self.three_row_spacing[after_row],
            _ => return None,
        };
        Some(if gap.is_finite() {
            gap.clamp(0.0, 60.0)
        } else {
            if rows == 3 { 3.0 } else { 4.0 }
        })
    }

    pub fn line_size(
        &self,
        source: crate::lyrics::Source,
        rows: usize,
        row: usize,
        base: f32,
    ) -> f32 {
        let scale = if self.font_scale.is_finite() {
            self.font_scale.clamp(0.5, 3.0)
        } else {
            1.0
        };
        let custom = if source == crate::lyrics::Source::Alsong {
            match rows {
                2 => self.two_row_sizes.get(row),
                3 => self.three_row_sizes.get(row),
                _ => None,
            }
            .copied()
            .unwrap_or(0.0)
        } else {
            0.0
        };
        if custom.is_finite() && custom > 0.0 {
            custom.clamp(12.0, 120.0)
        } else {
            base * scale
        }
    }
}

/// Secrets live in memory or the OS credential store, never in Settings or Debug output.
pub enum Request {
    ClearCache,
    Search {
        query: Query,
        page: u32,
    },
    Preview {
        query: Query,
        candidate: alsong::Candidate,
    },
    Translate {
        lyrics: Lyrics,
        config: translate::Config,
        api_key: String,
    },
    LoadKey(translate::Provider),
    SaveKey {
        provider: translate::Provider,
        api_key: String,
    },
}

pub enum Output {
    CacheCleared,
    Candidates(alsong::Candidates),
    Preview(Lyrics),
    Translation(Vec<translate::Line>),
    Key(String),
    KeySaved,
}

/// Each operation has its own request id; even a late answer for the same song is stale.
pub struct State {
    pub queue_open: bool,
    pub queue_pinned: bool,
    pub settings_section: usize,
    pub settings_open: bool,
    pub settings_draft: Option<Settings>,
    pub candidates_open: bool,
    pub search_terms: Option<(String, String)>,
    pub candidates: Loadable<alsong::Candidates>,
    pub preview: Loadable<Lyrics>,
    pub preview_candidate: Option<alsong::Candidate>,
    pub translated: Loadable<Vec<translate::Line>>,
    pub api_key: String,
    pub key_status: Option<String>,
    pub notice: Option<String>,
    pub lyrics_generation: u64,
    next_request: u64,
    pub search_request: Option<u64>,
    pub preview_request: Option<u64>,
    pub translation_request: Option<u64>,
    pub key_request: Option<u64>,
    pub cache_request: Option<u64>,
    pub cache_status: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            queue_open: false,
            queue_pinned: false,
            settings_section: 0,
            settings_open: false,
            settings_draft: None,
            candidates_open: false,
            search_terms: None,
            candidates: Loadable::NotLoaded,
            preview: Loadable::NotLoaded,
            preview_candidate: None,
            translated: Loadable::NotLoaded,
            api_key: String::new(),
            key_status: None,
            notice: None,
            lyrics_generation: 0,
            next_request: 0,
            search_request: None,
            preview_request: None,
            translation_request: None,
            key_request: None,
            cache_request: None,
            cache_status: None,
        }
    }
}

impl State {
    pub fn request_id(&mut self) -> u64 {
        self.next_request = self.next_request.wrapping_add(1);
        self.next_request
    }

    pub fn invalidate_translation(&mut self) {
        self.translation_request = None;
        self.translated = Loadable::NotLoaded;
    }

    pub fn change_track(&mut self) {
        self.lyrics_generation = self.lyrics_generation.wrapping_add(1);
        self.invalidate_translation();
        self.search_request = None;
        self.search_terms = None;
        self.preview_request = None;
        self.candidates = Loadable::NotLoaded;
        self.preview = Loadable::NotLoaded;
        self.preview_candidate = None;
        self.notice = None;
    }
}

pub async fn run(
    http: Result<reqwest::Client, String>,
    cache_dir: &std::path::Path,
    request: Request,
) -> Result<Output, String> {
    match request {
        Request::ClearCache => {
            let cache_dir = cache_dir.to_path_buf();
            tokio::task::spawn_blocking(move || match std::fs::remove_dir_all(cache_dir) {
                Ok(()) => Ok(Output::CacheCleared),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(Output::CacheCleared)
                }
                Err(error) => Err(error.to_string()),
            })
            .await
            .map_err(|error| error.to_string())?
        }
        Request::Search { query, page } => alsong::search(&http?, &query, page)
            .await
            .map(Output::Candidates),
        Request::Preview { query, candidate } => {
            alsong::preview(&http?, cache_dir, &query, &candidate)
                .await
                .and_then(|lyrics| {
                    lyrics.ok_or_else(|| "This candidate has no Korean lyrics.".into())
                })
                .map(Output::Preview)
        }
        Request::Translate {
            lyrics,
            config,
            api_key,
        } => translate::fetch(&http?, cache_dir, &lyrics, &config, &api_key)
            .await
            .map(Output::Translation),
        Request::LoadKey(provider) => key_operation(provider, None).await.map(Output::Key),
        Request::SaveKey { provider, api_key } => key_operation(provider, Some(api_key))
            .await
            .map(|_| Output::KeySaved),
    }
}

async fn key_operation(
    provider: translate::Provider,
    value: Option<String>,
) -> Result<String, String> {
    let task = tokio::task::spawn_blocking(move || {
        crate::credentials::ivlyrics_key(provider.key_name(), value)
    });
    tokio::time::timeout(std::time::Duration::from_secs(20), task)
        .await
        .map_err(|_| {
            "The credential store did not respond. Try again after unlocking it.".to_string()
        })?
        .map_err(|_| "The credential store is unavailable.".to_string())?
        .map_err(|_| {
            "Cannot access the system credential store. Unlock it and try again.".to_string()
        })
}

/// Read bounded external responses; neither provider errors nor credentials enter diagnostics.
pub(crate) async fn response_text(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, String> {
    if !response.status().is_success() {
        return Err(format!(
            "Provider returned HTTP {}. Check the service, key and quota; retry manually.",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("Provider response is too large.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read the provider response.")?
    {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("Provider response is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| "Provider returned invalid UTF-8.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn song_provider_round_trips_without_changing_defaults_or_old_choices() {
        use crate::lyrics::Source;
        let mut settings: Settings = serde_json::from_str(r#"{"tracks":{"old":{"offset_ms":700,"alsong":{"id":"123","title":"Title","artist":"Artist","album":""}}}}"#).unwrap();
        assert_eq!(settings.provider("old"), None);
        settings.selected_provider = Some(Source::Alsong);
        settings.tracks.entry("one".into()).or_default().provider = Some(Source::Spotify);
        settings.tracks.entry("auto".into()).or_default().provider = Some(Source::Unknown);
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.providers("one"), [Source::Spotify]);
        assert_eq!(restored.providers("auto"), restored.ordered_providers());
        assert_eq!(restored.providers("other"), [Source::Alsong]);
        assert_eq!(restored.offset("old"), 700);
        assert_eq!(restored.tracks["old"].alsong.as_ref().unwrap().id, "123");
    }

    #[tokio::test]
    async fn cache_clear_only_removes_lyrics_and_translation_and_reports_errors() {
        let root =
            std::env::temp_dir().join(format!("spotifast-lyrics-clear-{}", std::process::id()));
        let lyrics = root.join("lyrics");
        std::fs::create_dir_all(&lyrics).unwrap();
        for name in [
            "spotify-test.json",
            "alsong-test.json",
            "translation-test.json",
        ] {
            std::fs::write(lyrics.join(name), "cache").unwrap();
        }
        for name in ["settings.json", "audio", "art", "credentials"] {
            std::fs::write(root.join(name), "keep").unwrap();
        }
        for _ in 0..2 {
            assert!(matches!(
                run(Err("offline".into()), &lyrics, Request::ClearCache).await,
                Ok(Output::CacheCleared)
            ));
            assert!(!lyrics.exists());
        }
        for name in ["settings.json", "audio", "art", "credentials"] {
            assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), "keep");
        }
        std::fs::write(&lyrics, "not a directory").unwrap();
        assert!(
            run(Err("offline".into()), &lyrics, Request::ClearCache)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&lyrics).unwrap(), "not a directory");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn display_settings_keep_old_files_readable_and_bound_external_values() {
        let old: Settings =
            serde_json::from_str(r#"{"font_scale":1.4,"tracks":{"song":{"offset_ms":250}}}"#)
                .unwrap();
        assert_eq!(old.display, Display::default());
        let mut changed = old.clone();
        changed.display.reversed = true;
        changed.display.alignment = Alignment::Right;
        changed.display.show_album = false;
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored, changed);
        assert_eq!(restored.offset("song"), 250);
        let bounded = Display {
            album_size: f32::NAN,
            album_radius: -12.0,
            title_size: 1000.0,
            inactive_opacity: f32::INFINITY,
            translation_scale: -2.0,
            ..Display::default()
        }
        .validated();
        assert_eq!(bounded.album_size, Display::default().album_size);
        assert_eq!(bounded.album_radius, 0.0);
        assert_eq!(bounded.title_size, 64.0);
        assert_eq!(
            bounded.inactive_opacity,
            Display::default().inactive_opacity
        );
        assert_eq!(bounded.translation_scale, 0.4);
    }

    #[test]
    fn timing_offset_and_seek_are_inverse_with_saturation() {
        let mut settings = Settings::default();
        settings.tracks.insert(
            "track".into(),
            TrackSettings {
                offset_ms: 1500,
                ..Default::default()
            },
        );
        assert_eq!(settings.position("track", 1000), 0);
        assert_eq!(settings.position("track", 3000), 1500);
        assert_eq!(settings.seek_position("track", 1500), 3000);
        let lyrics = Lyrics {
            lines: crate::lyrics::parse_lrc("[00:00]start"),
            synced: true,
            ..Default::default()
        };
        assert_eq!(settings.active_line("track", &lyrics, 1000), None);
        assert_eq!(settings.active_line("track", &lyrics, 1500), Some(0));
        settings.tracks.get_mut("track").unwrap().offset_ms = -1500;
        assert_eq!(settings.position("track", 1000), 2500);
        assert_eq!(settings.seek_position("track", 1000), 0);
    }

    #[test]
    fn old_settings_default_to_original_lyrics_without_ai() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.ai_enabled);
        assert!(settings.alsong_first);
        assert!(
            !serde_json::to_string(&settings)
                .unwrap()
                .contains("api_key")
        );
    }

    #[test]
    fn provider_order_is_complete_unique_and_respects_manual_selection() {
        use crate::lyrics::Source::*;
        let mut settings = Settings::default();
        assert_eq!(settings.providers(""), [Alsong]);
        settings.selected_provider = None;
        assert_eq!(settings.providers(""), [Alsong, Spotify, Lrclib]);
        settings = serde_json::from_str(r#"{"alsong_first":false}"#).unwrap();
        assert_eq!(settings.providers(""), [Spotify, Lrclib, Alsong]);
        settings.provider_order = vec![Lrclib, Unknown, Lrclib, Alsong];
        assert_eq!(settings.providers(""), [Lrclib, Alsong, Spotify]);
        settings.selected_provider = Some(Spotify);
        assert_eq!(settings.providers(""), [Spotify]);
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
        settings.selected_provider = Some(Unknown);
        assert_eq!(settings.providers(""), [Lrclib, Alsong, Spotify]);
        let display = Display {
            line_spacing: f32::NAN,
            ..Default::default()
        }
        .validated();
        assert_eq!(display.line_spacing, 40.0);
        assert_eq!(
            Display {
                line_spacing: -10.0,
                ..display
            }
            .validated()
            .line_spacing,
            0.0
        );
        assert_eq!(
            Display {
                line_spacing: 999.0,
                ..display
            }
            .validated()
            .line_spacing,
            120.0
        );
    }

    #[test]
    fn key_actions_never_disclose_the_secret_in_debug_output() {
        let action = Action::Key(Secret("do-not-log-this-key".into()));
        assert!(!format!("{action:?}").contains("do-not-log"));
    }

    #[test]
    fn row_gaps_preserve_old_settings_and_validate_saved_values() {
        use crate::lyrics::Source;
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.two_row_sizes, [36.0, 38.0]);
        assert_eq!(settings.three_row_sizes, [36.0, 20.0, 38.0]);
        assert_eq!(settings.two_row_spacing, 4.0);
        assert_eq!(settings.three_row_spacing, [3.0, 3.0]);
        settings.two_row_spacing = 24.0;
        settings.three_row_spacing = [12.0, 30.0];
        assert_eq!(settings.row_spacing(Source::Alsong, 2, 0), Some(24.0));
        assert_eq!(settings.row_spacing(Source::Alsong, 3, 0), Some(12.0));
        assert_eq!(settings.row_spacing(Source::Alsong, 3, 1), Some(30.0));
        assert_eq!(settings.row_spacing(Source::Spotify, 3, 0), None);
        assert_eq!(settings.row_spacing(Source::Alsong, 3, 2), None);
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
        settings.three_row_spacing = [f32::NAN, 1000.0];
        assert_eq!(settings.row_spacing(Source::Alsong, 3, 0), Some(3.0));
        assert_eq!(settings.row_spacing(Source::Alsong, 3, 1), Some(60.0));
    }

    #[test]
    fn per_row_sizes_are_only_for_explicit_alsong_rows() {
        let settings = Settings {
            two_row_sizes: [36.0, 18.0],
            ..Default::default()
        };
        assert_eq!(
            settings.line_size(crate::lyrics::Source::Alsong, 2, 1, 24.0),
            18.0
        );
        assert_eq!(
            settings.line_size(crate::lyrics::Source::Spotify, 2, 1, 24.0),
            24.0
        );
        assert_eq!(
            settings.line_size(crate::lyrics::Source::Alsong, 4, 1, 24.0),
            24.0
        );
    }

    #[test]
    fn track_change_discards_all_pending_lyric_work() {
        let mut state = State::default();
        state.translation_request = Some(state.request_id());
        state.preview_request = Some(state.request_id());
        state.search_request = Some(state.request_id());
        state.search_terms = Some(("Alternative title".into(), String::new()));
        state.change_track();
        assert_eq!(state.translation_request, None);
        assert_eq!(state.preview_request, None);
        assert_eq!(state.search_request, None);
        assert_eq!(state.search_terms, None);
        assert_eq!(state.lyrics_generation, 1);
    }
}
