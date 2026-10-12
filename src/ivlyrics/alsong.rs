//! ALSong matching and SOAP protocol adapted from the user's ivLyrics patch.
//! Modified 2026-10-09: ported to native Rust for this local Spotifast edition.
//! SPDX-License-Identifier: LGPL-2.1-only
//! See `docs/ivlyrics.md` and `assets/ivlyrics/LICENSE.LGPL-2.1` for attribution.

use std::time::Duration;

use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::lyrics::{Line, Lyrics, Query, Source};

const API: &str = "http://lyrics.alsong.co.kr/alsongwebservice/service1.asmx";
const ENC_DATA: &str = "7c2d15b8f51ac2f3b2a37d7a445c3158455defb8a58d621eb77a3ff8ae4921318e49cefe24e515f79892a4c29c9a3e204358698c1cfe79c151c04f9561e945096ccd1d1c0a8d8f265a2f3fa7995939b21d8f663b246bbc433c7589da7e68047524b80e16f9671b6ea0faaf9d6cde1b7dbcf1b89aa8a1d67a8bbc566664342e12";
const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
}

impl Candidate {
    pub fn valid_id(&self) -> bool {
        !self.id.is_empty()
            && self.id.len() <= 20
            && !self.id.starts_with('0')
            && self.id.bytes().all(|c| c.is_ascii_digit())
    }
}

#[derive(Clone, Debug)]
pub struct Candidates {
    pub page: u32,
    pub entries: Vec<Candidate>,
    pub has_more: bool,
}

fn normalize(text: &str) -> String {
    text.nfkc()
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn aliases(text: &str) -> Vec<String> {
    let mut names = vec![normalize(text)];
    let mut outside = String::new();
    let mut inside = String::new();
    let mut parenthesized = false;
    for c in text.nfkc() {
        match c {
            '(' => {
                parenthesized = true;
                inside.clear();
                outside.push(' ');
            }
            ')' => {
                parenthesized = false;
                names.push(normalize(&inside));
            }
            _ if parenthesized => inside.push(c),
            _ => outside.push(c),
        }
    }
    names.push(normalize(&outside));
    let collaborations = text
        .to_lowercase()
        .replace(" featuring ", ";")
        .replace(" feat. ", ";")
        .replace(" feat ", ";")
        .replace(" & ", ";");
    names.extend(collaborations.split([',', ';']).map(normalize));
    names.retain(|name| !name.is_empty());
    names.sort();
    names.dedup();
    names
}

pub fn matches(candidate: &Candidate, query: &Query) -> bool {
    let artists = aliases(&query.artist);
    !normalize(&query.title).is_empty()
        && candidate.valid_id()
        && normalize(&candidate.title) == normalize(&query.title)
        && aliases(&candidate.artist)
            .iter()
            .any(|name| artists.contains(name))
}

fn rank(entries: &mut Vec<Candidate>, query: &Query) {
    let mut seen = std::collections::HashSet::new();
    entries.retain(|candidate| matches(candidate, query) && seen.insert(candidate.id.clone()));
    entries.sort_by_key(|candidate| {
        std::cmp::Reverse(
            u8::from(normalize(&candidate.artist) == normalize(&query.artist)) * 2
                + u8::from(
                    !query.album.is_empty()
                        && normalize(&candidate.album) == normalize(&query.album),
                ),
        )
    });
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn envelope(action: &str, fields: &[(&str, &str)]) -> String {
    let mut body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\"><soap:Body><{action} xmlns=\"ALSongWebServer\"><encData>{ENC_DATA}</encData>"
    );
    for (key, value) in fields {
        body.push_str(&format!("<{key}>{}</{key}>", xml_escape(value)));
    }
    body.push_str(&format!("</{action}></soap:Body></soap:Envelope>"));
    body
}

async fn soap(
    http: &reqwest::Client,
    action: &str,
    fields: &[(&str, &str)],
) -> Result<String, String> {
    let response = http
        .post(API)
        .timeout(DEADLINE)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", format!("ALSongWebServer/{action}"))
        .header("User-Agent", "gSOAP/2.7")
        .body(envelope(action, fields))
        .send()
        .await
        .map_err(|_| "ALSong could not be reached or timed out.".to_string())?;
    super::response_text(response, 2 * 1024 * 1024).await
}

fn document<'a>(xml: &'a str, action: &str) -> Result<Document<'a>, String> {
    let doc = Document::parse(xml).map_err(|_| "Invalid ALSong XML response.")?;
    let body = doc
        .descendants()
        .find(|node| node.has_tag_name("Body"))
        .ok_or("Missing ALSong SOAP body.")?;
    if body.descendants().any(|node| node.has_tag_name("Fault")) {
        return Err("ALSong returned a SOAP fault.".into());
    }
    let expected = format!("{action}Response");
    if !body
        .children()
        .any(|node| node.has_tag_name(expected.as_str()))
    {
        return Err("Missing ALSong SOAP response.".into());
    }
    Ok(doc)
}

fn field(root: Node<'_, '_>, name: &str) -> String {
    root.descendants()
        .find(|node| node.has_tag_name(name))
        .map(|node| {
            node.descendants()
                .filter(|node| node.is_text())
                .filter_map(|node| node.text())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

fn parse_search(xml: &str, page: u32) -> Result<Candidates, String> {
    let doc = document(xml, "GetResembleLyricList2")?;
    let root = doc
        .descendants()
        .find(|node| node.has_tag_name("GetResembleLyricList2Result"))
        .ok_or("Missing ALSong search result.")?;
    let mut entries: Vec<_> = root
        .descendants()
        .filter(|node| node.has_tag_name("ST_SEARCHLYRIC_LIST"))
        .map(|node| Candidate {
            id: field(node, "lyricID"),
            title: field(node, "title"),
            artist: field(node, "artist"),
            album: field(node, "album"),
        })
        .collect();
    // The API has no total count. One final empty page ends manual browsing.
    let has_more = !entries.is_empty();
    let mut seen = std::collections::HashSet::new();
    entries.retain(|candidate| candidate.valid_id() && seen.insert(candidate.id.clone()));
    Ok(Candidates {
        page,
        entries,
        has_more,
    })
}

async fn search_page(
    http: &reqwest::Client,
    query: &Query,
    artist: &str,
    page: u32,
) -> Result<Candidates, String> {
    let page_text = page.to_string();
    let xml = soap(
        http,
        "GetResembleLyricList2",
        &[
            ("title", &query.title),
            ("artist", artist),
            ("pageNo", &page_text),
        ],
    )
    .await?;
    parse_search(&xml, page)
}

pub async fn search(
    http: &reqwest::Client,
    query: &Query,
    page: u32,
) -> Result<Candidates, String> {
    if (query.title.trim().is_empty() && query.artist.trim().is_empty())
        || !(1..=100).contains(&page)
    {
        return Err("A title or artist and valid page are required.".into());
    }
    tokio::time::timeout(DEADLINE, search_page(http, query, &query.artist, page))
        .await
        .map_err(|_| "ALSong search timed out.".to_string())?
}

pub async fn fetch(
    http: &reqwest::Client,
    query: &Query,
    selected: Option<&Candidate>,
) -> Result<Option<Lyrics>, String> {
    tokio::time::timeout(DEADLINE, async {
        if let Some(candidate) = selected {
            return detail_inner(http, query, candidate, true).await;
        }
        if query.title.trim().is_empty() || query.artist.trim().is_empty() {
            return Ok(None);
        }
        let mut candidates = search_page(http, query, &query.artist, 1).await?;
        rank(&mut candidates.entries, query);
        if candidates.entries.is_empty() {
            candidates = search_page(http, query, "", 1).await?;
            rank(&mut candidates.entries, query);
        }
        for candidate in candidates.entries.iter().take(3) {
            if let Some(lyrics) = detail_inner(http, query, candidate, false).await? {
                return Ok(Some(lyrics));
            }
        }
        Ok(None)
    })
    .await
    .map_err(|_| "ALSong lookup timed out.".to_string())?
}

pub async fn cached_fetch(
    http: &reqwest::Client,
    cache_dir: &std::path::Path,
    query: &Query,
    selected: Option<&Candidate>,
) -> Result<Option<Lyrics>, String> {
    if selected.is_some_and(|candidate| !candidate.valid_id()) {
        return Err("Invalid saved ALSong candidate ID.".into());
    }
    let id = selected.map_or("auto", |candidate| candidate.id.as_str());
    let path = cache_path(cache_dir, query, id);
    if let Some(lyrics) = crate::lyrics::cached(&path) {
        return Ok(lyrics);
    }
    let found = fetch(http, query, selected).await?;
    crate::lyrics::store(&path, &found);
    Ok(found)
}

fn cache_path(cache_dir: &std::path::Path, query: &Query, id: &str) -> std::path::PathBuf {
    cache_dir.join(format!(
        "alsong-v1-{}-{id}.json",
        crate::lyrics::cache_key(query)
    ))
}

pub async fn preview(
    http: &reqwest::Client,
    cache_dir: &std::path::Path,
    query: &Query,
    candidate: &Candidate,
) -> Result<Option<Lyrics>, String> {
    let found = tokio::time::timeout(DEADLINE, detail_inner(http, query, candidate, true))
        .await
        .map_err(|_| "ALSong lyrics timed out.".to_string())??;
    // A freshly chosen preview must remain the same on the next play, even if an older copy was cached.
    if found.is_some() {
        crate::lyrics::store(&cache_path(cache_dir, query, &candidate.id), &found);
    }
    Ok(found)
}

async fn detail_inner(
    http: &reqwest::Client,
    query: &Query,
    candidate: &Candidate,
    selected: bool,
) -> Result<Option<Lyrics>, String> {
    if !candidate.valid_id() || (!selected && !matches(candidate, query)) {
        return Err("The ALSong candidate does not match this song.".into());
    }
    let xml = soap(http, "GetLyricByID2", &[("lyricID", &candidate.id)]).await?;
    parse_detail(&xml, query, candidate, selected)
}

fn parse_detail(
    xml: &str,
    query: &Query,
    candidate: &Candidate,
    selected: bool,
) -> Result<Option<Lyrics>, String> {
    let doc = document(xml, "GetLyricByID2")?;
    let output = doc
        .descendants()
        .find(|node| node.has_tag_name("output"))
        .ok_or("Missing ALSong lyric output.")?;
    let id = field(output, "lyricID");
    if !id.is_empty() && id != candidate.id {
        return Err("ALSong returned a different lyric ID.".into());
    }
    let seconds = field(output, "playtime").parse::<f64>().unwrap_or(0.0);
    let duration = if seconds.is_finite() && seconds > 0.0 {
        Some(seconds * 1000.0)
    } else {
        None
    };
    let lyrics = parse_lyrics(&field(output, "lyric"));
    if !lyrics
        .lines
        .iter()
        .any(|line| line.text.chars().any(is_hangul))
    {
        return Ok(None);
    }
    let length = duration.or_else(|| {
        lyrics
            .lines
            .last()
            .and_then(|line| line.at_ms)
            .filter(|at| *at > 0)
            .map(f64::from)
    });
    if !selected
        && query.duration_ms > 0
        && length.is_some_and(|ms| (ms - f64::from(query.duration_ms)).abs() >= 60_000.0)
    {
        return Ok(None);
    }
    Ok(Some(lyrics))
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '\u{a960}'..='\u{a97f}' | '\u{ac00}'..='\u{d7af}' | '\u{d7b0}'..='\u{d7ff}')
}

pub fn parse_lyrics(raw: &str) -> Lyrics {
    // ALSong uses HTML break tags inside its XML string, not arbitrary HTML.
    let raw = raw.trim_start_matches('\u{feff}').replace('\r', "\n");
    let mut parts = raw.split('<');
    let mut text = parts.next().unwrap_or_default().to_string();
    for part in parts {
        if let Some((tag, rest)) = part.split_once('>')
            && tag
                .trim()
                .trim_end_matches('/')
                .trim()
                .eq_ignore_ascii_case("br")
        {
            text.push('\n');
            text.push_str(rest);
        } else {
            text.push('<');
            text.push_str(part);
        }
    }
    let offset = text
        .lines()
        .find_map(|line| {
            let lower = line.trim().to_ascii_lowercase();
            lower
                .strip_prefix("[offset:")?
                .strip_suffix(']')?
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0)
        .clamp(-86_400_000, 86_400_000);
    let mut timed = crate::lyrics::parse_lrc(&text);
    for line in &mut timed {
        line.at_ms = line
            .at_ms
            .map(|at| (i64::from(at) + offset).clamp(0, i64::from(u32::MAX)) as u32);
    }
    if timed.iter().any(|line| !line.text.is_empty()) {
        let mut merged: Vec<Line> = Vec::new();
        for line in timed {
            if let Some(previous) = merged
                .last_mut()
                .filter(|previous| previous.at_ms == line.at_ms)
            {
                if !line.text.is_empty() && !previous.text.lines().any(|text| text == line.text) {
                    if !previous.text.is_empty() {
                        previous.text.push('\n');
                    }
                    previous.text.push_str(&line.text);
                }
            } else {
                merged.push(line);
            }
        }
        Lyrics {
            lines: merged,
            synced: true,
            instrumental: false,
            source: Source::Alsong,
        }
    } else {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('['))
            .map(|text| Line {
                at_ms: None,
                text: text.to_string(),
            })
            .collect();
        Lyrics {
            lines,
            synced: false,
            instrumental: false,
            source: Source::Alsong,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_fresh_preview_replaces_the_saved_candidates_old_cache() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 1024];
            loop {
                let read = socket.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&buffer[..read]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length: usize = header
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = "<Body><GetLyricByID2Response><output><lyricID>123</lyricID><playtime>180</playtime><lyric><![CDATA[[00:01]새 가사]]></lyric></output></GetLyricByID2Response></Body>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let http = reqwest::Client::builder()
            .no_proxy()
            .proxy(reqwest::Proxy::all(format!("http://{address}")).unwrap())
            .build()
            .unwrap();
        let directory = std::env::temp_dir().join(format!(
            "ivlyrics-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = cache_path(&directory, &query(), "123");
        crate::lyrics::store(&path, &Some(parse_lyrics("[00:01]오래된 가사")));
        let manual = Candidate {
            title: "Different spelling".into(),
            artist: "Another alias".into(),
            ..candidate()
        };
        let fresh = preview(&http, &directory, &query(), &manual)
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert_eq!(fresh.lines[0].text, "새 가사");
        // The server is gone: replay must use the newly previewed words from disk.
        assert_eq!(
            cached_fetch(&http, &directory, &query(), Some(&manual))
                .await
                .unwrap(),
            Some(fresh)
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[tokio::test]
    #[ignore = "queries a public ALSong sample; requires the live service"]
    async fn live_alsong_lookup_returns_korean_timed_lyrics() {
        let http = crate::http::build_client(&crate::settings::ProxyConfig::System).unwrap();
        let query = Query {
            title: "좋은 날".into(),
            artist: "아이유 (IU)".into(),
            album: "Real".into(),
            duration_ms: 233_000,
        };
        let found = fetch(&http, &query, None)
            .await
            .unwrap()
            .expect("public sample lyrics");
        assert!(found.synced);
        assert!(
            found
                .lines
                .iter()
                .any(|line| line.text.chars().any(is_hangul))
        );
        assert_eq!(found.source, Source::Alsong);
    }

    fn query() -> Query {
        Query {
            title: "좋은 날".into(),
            artist: "아이유 (IU)".into(),
            album: "Real".into(),
            duration_ms: 180_000,
        }
    }
    fn candidate() -> Candidate {
        Candidate {
            id: "123".into(),
            title: "좋은 날".into(),
            artist: "IU".into(),
            album: "Real".into(),
        }
    }

    #[test]
    fn matching_accepts_aliases_but_rejects_covers_and_bad_ids() {
        assert!(matches(&candidate(), &query()));
        assert_eq!(normalize("ＡＢＣ　１２"), "abc 12");
        let mut candidate = candidate();
        candidate.artist = "Cover band".into();
        assert!(!matches(&candidate, &query()));
        candidate.artist = "IU".into();
        candidate.id = "0<script>".into();
        assert!(!matches(&candidate, &query()));
    }

    #[test]
    fn manual_results_keep_alternative_names_while_automatic_matching_stays_strict() {
        let xml = "<Body><GetResembleLyricList2Response><GetResembleLyricList2Result><ST_SEARCHLYRIC_LIST><lyricID>123</lyricID><title>Another spelling</title><artist>Another name</artist></ST_SEARCHLYRIC_LIST><ST_SEARCHLYRIC_LIST><lyricID>../../bad</lyricID><title>Bad</title></ST_SEARCHLYRIC_LIST></GetResembleLyricList2Result></GetResembleLyricList2Response></Body>";
        let mut results = parse_search(xml, 1).unwrap();
        assert_eq!(results.entries.len(), 1);
        assert_eq!(results.entries[0].id, "123");
        rank(&mut results.entries, &query());
        assert!(results.entries.is_empty());
    }

    #[test]
    fn timestamps_offsets_breaks_and_same_time_rows_are_preserved() {
        let lyrics = parse_lyrics(
            "[offset:-100]\r\n[00:00.1]시작<br />[00:01:25]원문<br>[00:01.250]번역\n[00:02.5][00:03.50]반복\n[00:04.000]",
        );
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines[0].at_ms, Some(0));
        assert_eq!(lyrics.lines[1].at_ms, Some(1150));
        assert_eq!(lyrics.lines[1].text, "원문\n번역");
        assert_eq!(lyrics.lines[2].at_ms, Some(2400));
        assert_eq!(lyrics.lines[3].at_ms, Some(3400));
        assert_eq!(lyrics.lines[4].text, "");
    }

    #[test]
    fn xml_is_namespace_independent_and_rejects_faults_and_dtds() {
        let xml = "<s:Envelope xmlns:s='urn:soap'><s:Body><GetResembleLyricList2Response><GetResembleLyricList2Result><ST_SEARCHLYRIC_LIST><lyricID>123</lyricID><title>좋은 날</title><artist>IU</artist><album>Real &amp; More</album></ST_SEARCHLYRIC_LIST></GetResembleLyricList2Result></GetResembleLyricList2Response></s:Body></s:Envelope>";
        assert_eq!(
            parse_search(xml, 1).unwrap().entries[0].album,
            "Real & More"
        );
        assert!(document("<Body><Fault/></Body>", "GetLyricByID2").is_err());
        assert!(
            document(
                "<!DOCTYPE x [<!ENTITY y 'oops'>]><Body>&y;</Body>",
                "GetLyricByID2"
            )
            .is_err()
        );
        assert!(envelope("Search", &[("title", "<&\"'")]).contains("&lt;&amp;&quot;&apos;"));
    }

    #[test]
    fn automatic_selection_requires_korean_and_plausible_duration() {
        let xml = |lyric: &str, playtime| {
            format!(
                "<Body><GetLyricByID2Response><output><lyricID>123</lyricID><playtime>{playtime}</playtime><lyric><![CDATA[{lyric}]]></lyric></output></GetLyricByID2Response></Body>"
            )
        };
        assert!(
            parse_detail(&xml("[00:01]한글", 180), &query(), &candidate(), false)
                .unwrap()
                .is_some()
        );
        assert!(
            parse_detail(&xml("[00:01]English", 180), &query(), &candidate(), true)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_detail(&xml("[00:01]한글", 240), &query(), &candidate(), false)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_detail(&xml("[00:01]한글", 240), &query(), &candidate(), true)
                .unwrap()
                .is_some()
        );
        assert!(
            parse_detail(&xml("[00:01]한글", 0), &query(), &candidate(), false)
                .unwrap()
                .is_none()
        );
    }
}
