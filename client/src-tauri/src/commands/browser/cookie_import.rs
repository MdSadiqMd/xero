use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{
    webview::{cookie, Cookie},
    AppHandle, Manager, Runtime, State,
};
use time::{Duration, OffsetDateTime};

use crate::commands::{CommandError, CommandResult};

use super::tabs::BrowserTabs;
use super::BrowserState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrowserSource {
    Chrome,
    Chromium,
    Brave,
    Edge,
    Opera,
    OperaGx,
    Vivaldi,
    Arc,
    Firefox,
    LibreWolf,
    Zen,
    #[cfg(target_os = "macos")]
    Safari,
}

impl BrowserSource {
    fn label(self) -> &'static str {
        match self {
            BrowserSource::Chrome => "Google Chrome",
            BrowserSource::Chromium => "Chromium",
            BrowserSource::Brave => "Brave",
            BrowserSource::Edge => "Microsoft Edge",
            BrowserSource::Opera => "Opera",
            BrowserSource::OperaGx => "Opera GX",
            BrowserSource::Vivaldi => "Vivaldi",
            BrowserSource::Arc => "Arc",
            BrowserSource::Firefox => "Firefox",
            BrowserSource::LibreWolf => "LibreWolf",
            BrowserSource::Zen => "Zen",
            #[cfg(target_os = "macos")]
            BrowserSource::Safari => "Safari",
        }
    }

    fn id(self) -> &'static str {
        match self {
            BrowserSource::Chrome => "chrome",
            BrowserSource::Chromium => "chromium",
            BrowserSource::Brave => "brave",
            BrowserSource::Edge => "edge",
            BrowserSource::Opera => "opera",
            BrowserSource::OperaGx => "opera_gx",
            BrowserSource::Vivaldi => "vivaldi",
            BrowserSource::Arc => "arc",
            BrowserSource::Firefox => "firefox",
            BrowserSource::LibreWolf => "librewolf",
            BrowserSource::Zen => "zen",
            #[cfg(target_os = "macos")]
            BrowserSource::Safari => "Safari",
        }
    }

    fn all() -> Vec<BrowserSource> {
        let mut sources = vec![
            BrowserSource::Chrome,
            BrowserSource::Chromium,
            BrowserSource::Brave,
            BrowserSource::Edge,
            BrowserSource::Opera,
            BrowserSource::OperaGx,
            BrowserSource::Vivaldi,
            BrowserSource::Arc,
            BrowserSource::Firefox,
            BrowserSource::LibreWolf,
            BrowserSource::Zen,
        ];
        #[cfg(target_os = "macos")]
        sources.push(BrowserSource::Safari);
        sources
    }

    fn fetch(self, domains: Option<Vec<String>>) -> rookie::Result<Vec<rookie::enums::Cookie>> {
        match self {
            BrowserSource::Chrome => rookie::chrome(domains),
            BrowserSource::Chromium => rookie::chromium(domains),
            BrowserSource::Brave => rookie::brave(domains),
            BrowserSource::Edge => rookie::edge(domains),
            BrowserSource::Opera => rookie::opera(domains),
            BrowserSource::OperaGx => rookie::opera_gx(domains),
            BrowserSource::Vivaldi => rookie::vivaldi(domains),
            BrowserSource::Arc => rookie::arc(domains),
            BrowserSource::Firefox => rookie::firefox(domains),
            BrowserSource::LibreWolf => rookie::librewolf(domains),
            BrowserSource::Zen => rookie::zen(domains),
            #[cfg(target_os = "macos")]
            BrowserSource::Safari => rookie::safari(domains),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectedBrowser {
    pub id: String,
    pub label: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CookieImportResult {
    pub source: String,
    pub imported: u32,
    pub skipped: u32,
    pub domains: u32,
}

#[tauri::command]
pub fn browser_list_cookie_sources<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, BrowserState>,
) -> CommandResult<Vec<DetectedBrowser>> {
    // Probe each browser with a cheap call (domains filter = []) — if rookie can
    // resolve the config paths and open the DB, we consider the source
    // available. Read failures (no install, locked DB, missing keychain entry)
    // surface as `available: false`.
    let mut out = Vec::new();
    for source in BrowserSource::all() {
        let available = source.fetch(Some(vec!["__cadence_probe__.invalid".to_string()])).is_ok();
        out.push(DetectedBrowser {
            id: source.id().to_string(),
            label: source.label().to_string(),
            available,
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn browser_import_cookies<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, BrowserState>,
    source: String,
    domains: Option<Vec<String>>,
) -> CommandResult<CookieImportResult> {
    let source_enum = parse_source(&source)?;
    let domains = domains.and_then(|list| {
        let cleaned: Vec<String> = list
            .into_iter()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .collect();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    });

    let cookies = source_enum.fetch(domains).map_err(|error| {
        CommandError::user_fixable(
            "browser_cookie_read_failed",
            format!(
                "Could not read cookies from {label}: {error}. On macOS you may need to grant Full Disk Access; on newer systems Cadence also needs permission to access that browser's data directory.",
                label = source_enum.label(),
            ),
        )
    })?;

    let tabs = state.tabs();
    let webview = resolve_any_webview(&app, &tabs)?;

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut unique_domains = std::collections::HashSet::new();

    for raw in cookies {
        unique_domains.insert(raw.domain.clone());
        match build_cookie(&raw) {
            Some(cookie) => match webview.set_cookie(cookie) {
                Ok(()) => imported += 1,
                Err(_) => skipped += 1,
            },
            None => {
                skipped += 1;
            }
        }
    }

    Ok(CookieImportResult {
        source: source_enum.id().to_string(),
        imported,
        skipped,
        domains: unique_domains.len() as u32,
    })
}

fn parse_source(raw: &str) -> CommandResult<BrowserSource> {
    for candidate in BrowserSource::all() {
        if candidate.id().eq_ignore_ascii_case(raw) {
            return Ok(candidate);
        }
    }
    Err(CommandError::user_fixable(
        "browser_cookie_source_unknown",
        format!("Unknown cookie source `{raw}`."),
    ))
}

fn resolve_any_webview<R: Runtime>(
    app: &AppHandle<R>,
    tabs: &Arc<BrowserTabs>,
) -> CommandResult<tauri::webview::Webview<R>> {
    // Cookies set on any of our webviews populate the shared WKWebsiteDataStore
    // / WebView2 cookie manager, so one live webview is enough to stage the
    // import for every tab (current and future).
    let list = tabs.list()?;
    for tab in list {
        if let Some(webview) = app.get_webview(&tab.label) {
            return Ok(webview);
        }
    }
    Err(CommandError::user_fixable(
        "browser_not_open",
        "Open a page in the in-app browser before importing cookies.",
    ))
}

fn build_cookie(raw: &rookie::enums::Cookie) -> Option<Cookie<'static>> {
    if raw.name.is_empty() {
        return None;
    }

    let domain = raw.domain.trim_matches('.').to_string();
    if domain.is_empty() {
        return None;
    }
    let path = if raw.path.is_empty() {
        "/".to_string()
    } else {
        raw.path.clone()
    };

    let same_site = match raw.same_site {
        0 => Some(cookie::SameSite::None),
        1 => Some(cookie::SameSite::Lax),
        2 => Some(cookie::SameSite::Strict),
        _ => None,
    };

    let mut builder = Cookie::build((raw.name.clone(), raw.value.clone()))
        .domain(domain)
        .path(path)
        .secure(raw.secure)
        .http_only(raw.http_only);

    if let Some(ss) = same_site {
        builder = builder.same_site(ss);
    }

    if let Some(expires) = raw.expires {
        if let Ok(dt) = OffsetDateTime::from_unix_timestamp(expires as i64) {
            builder = builder.expires(cookie::Expiration::DateTime(dt));
        }
    } else {
        // Session cookie — mark as expiring far in the future so wry persists
        // it for this session. Without an explicit expiry, WKHTTPCookieStore
        // may drop it across webview tears.
        let expires = OffsetDateTime::now_utc() + Duration::days(30);
        builder = builder.expires(cookie::Expiration::DateTime(expires));
    }

    Some(builder.build())
}
