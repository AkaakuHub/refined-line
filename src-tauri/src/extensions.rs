use crate::config::load_config;
use crate::crx::{
  build_update_url, check_update, download_crx, ensure_clean_dir, extract_zip, inject_manifest_key,
  parse_crx3, UpdateCheck,
};
use crate::paths::app_data_root;
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

#[cfg(target_os = "windows")]
use crate::windowing::{attach_new_window_handler, attach_permission_handler};
#[cfg(target_os = "windows")]
use std::sync::mpsc;
#[cfg(target_os = "windows")]
use tauri::webview::PlatformWebview;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
  ICoreWebView2, ICoreWebView2BrowserExtension, ICoreWebView2Profile7, ICoreWebView2_13,
  ICoreWebView2_2, COREWEBVIEW2_COOKIE_SAME_SITE_KIND,
};
#[cfg(target_os = "windows")]
use webview2_com::{
  take_pwstr, wait_with_pump, BrowserExtensionEnableCompletedHandler, GetCookiesCompletedHandler,
  ProfileAddBrowserExtensionCompletedHandler,
};
#[cfg(target_os = "windows")]
use windows::core::BOOL;
#[cfg(target_os = "windows")]
use windows::core::{Interface, HSTRING, PCWSTR, PWSTR};

pub(crate) struct ExtensionSetup {
  pub(crate) line_dir: PathBuf,
  pub(crate) user_dir: PathBuf,
  pub(crate) updated: bool,
  pub(crate) update_failed: bool,
}

pub(crate) fn prepare_extensions(app: &tauri::AppHandle) -> Result<ExtensionSetup> {
  let config = load_config(app)?;
  let app_data = app_data_root(app)?;

  let extensions_root = app_data.join("extensions");
  let line_dir = extensions_root.join("line");
  let user_dir = extensions_root.join("user");

  info!("[update] storage root={}", extensions_root.display());
  fs::create_dir_all(&user_dir)?;

  let current_version = read_manifest_version(&line_dir);
  let update_url = build_update_url(
    &config.update2_base_url,
    &config.line_extension_id,
    current_version.as_deref(),
  );
  let has_existing = is_extension_dir(&line_dir);

  let mut updated = false;
  let mut update_failed = false;
  let mut crx_bytes: Option<Vec<u8>> = None;

  if let Some(version) = current_version.as_deref() {
    info!("[update] check v{} {}", version, update_url);
    match check_update(&update_url) {
      Ok(UpdateCheck::NoUpdate) => {
        if has_existing {
          info!("[update] use local extension (v{})", version);
          let _ = disable_cache_clear(&line_dir);
          let _ = disable_legacy_clear(&line_dir);
          return Ok(ExtensionSetup {
            line_dir,
            user_dir,
            updated: false,
            update_failed: false,
          });
        }
      }
      Ok(UpdateCheck::UpdateAvailable(payload)) => {
        info!("[update] update available");
        updated = has_existing;
        crx_bytes = payload;
      }
      Err(error) => {
        warn!("[update] check failed: {error:#}");
      }
    }
  }

  if crx_bytes.is_none() {
    info!("[update] download {}", update_url);
    match download_crx_with_retry(&update_url) {
      Ok(buffer) => {
        crx_bytes = Some(buffer);
      }
      Err(error) => {
        warn!("[update] download failed: {error:#}");
        update_failed = true;
      }
    }
  }

  if update_failed {
    if has_existing {
      info!("[update] use local extension (update failed)");
      let _ = disable_cache_clear(&line_dir);
      let _ = disable_legacy_clear(&line_dir);
      return Ok(ExtensionSetup {
        line_dir,
        user_dir,
        updated: false,
        update_failed: true,
      });
    }
    return Err(anyhow!("update download failed after retries"));
  }

  let crx_bytes = crx_bytes.ok_or_else(|| anyhow!("crx bytes missing"))?;
  let parsed = parse_crx3(&crx_bytes)?;
  ensure_clean_dir(&line_dir)?;
  extract_zip(&parsed.zip_bytes, &line_dir)?;
  inject_manifest_key(&line_dir, &parsed.public_key)?;
  let _ = disable_cache_clear(&line_dir);
  let _ = disable_legacy_clear(&line_dir);
  if let Some(version) = read_manifest_version(&line_dir) {
    info!("[update] installed extension v{} (network)", version);
  } else {
    info!("[update] installed extension (network)");
  }

  Ok(ExtensionSetup {
    line_dir,
    user_dir,
    updated,
    update_failed: false,
  })
}

#[cfg(target_os = "windows")]
fn add_browser_extension(
  profile: &ICoreWebView2Profile7,
  extension_dir: &Path,
) -> Result<ICoreWebView2BrowserExtension> {
  let path = extension_dir.canonicalize()?;
  let path_hs = HSTRING::from(path.as_path());
  let (tx, rx) = mpsc::channel();
  let handler =
    ProfileAddBrowserExtensionCompletedHandler::create(Box::new(move |result, extension| {
      let _ = tx.send((result, extension));
      Ok(())
    }));

  unsafe {
    profile.AddBrowserExtension(&path_hs, &handler)?;
  }

  let (result, extension) =
    wait_with_pump(rx).map_err(|error| anyhow!("extension install callback error: {error:?}"))?;

  if let Err(error) = result {
    return Err(anyhow!("add extension failed: {error:?}"));
  }

  extension.ok_or_else(|| anyhow!("add extension returned no extension"))
}

#[cfg(target_os = "windows")]
fn browser_extension_id(extension: &ICoreWebView2BrowserExtension) -> Result<String> {
  let mut id_ptr = PWSTR::null();
  unsafe {
    extension.Id(&mut id_ptr)?;
  }
  Ok(take_pwstr(id_ptr))
}

#[cfg(target_os = "windows")]
fn ensure_extension_enabled(extension: &ICoreWebView2BrowserExtension) -> Result<()> {
  let mut enabled = BOOL(0);
  unsafe {
    extension.IsEnabled(&mut enabled)?;
  }
  if enabled == true {
    return Ok(());
  }

  let (tx, rx) = mpsc::channel();
  let handler = BrowserExtensionEnableCompletedHandler::create(Box::new(move |result| {
    let _ = tx.send(result);
    Ok(())
  }));

  unsafe {
    extension.Enable(true, &handler)?;
  }

  let result =
    wait_with_pump(rx).map_err(|error| anyhow!("extension enable callback error: {error:?}"))?;
  if let Err(error) = result {
    return Err(anyhow!("enable extension failed: {error:?}"));
  }

  Ok(())
}

fn is_extension_dir(path: &Path) -> bool {
  path.join("manifest.json").is_file()
}

fn read_manifest_version(path: &Path) -> Option<String> {
  let manifest_path = path.join("manifest.json");
  let raw = fs::read_to_string(&manifest_path).ok()?;
  let value: Value = serde_json::from_str(&raw).ok()?;
  value
    .get("version")
    .and_then(|v| v.as_str())
    .map(|v| v.to_string())
}

fn disable_cache_clear(line_dir: &Path) -> Result<()> {
  let cache_path = line_dir.join("cache.js");
  if !cache_path.is_file() {
    return Ok(());
  }
  let raw = fs::read_to_string(&cache_path)?;
  if raw.contains("caches.delete(CACHE_NAME)") {
    let updated = raw.replace("caches.delete(CACHE_NAME)", "Promise.resolve()");
    fs::write(&cache_path, updated)?;
  }
  Ok(())
}

fn disable_legacy_clear(line_dir: &Path) -> Result<()> {
  let background_path = line_dir.join("background.js");
  if !background_path.is_file() {
    return Ok(());
  }
  let raw = fs::read_to_string(&background_path)?;
  let mut updated = raw.clone();
  let mut changed = false;
  if raw.contains("chrome.storage.local.clear()") {
    updated = updated.replace("chrome.storage.local.clear()", "Promise.resolve()");
    changed = true;
  }
  if raw.contains("indexedDB.databases()") {
    updated = updated.replace("indexedDB.databases()", "Promise.resolve([])");
    changed = true;
  }
  if changed {
    fs::write(&background_path, updated)?;
  }
  Ok(())
}

fn download_crx_with_retry(url: &str) -> Result<Vec<u8>> {
  const RETRIES: usize = 5;
  const WAIT_SECS: u64 = 30;
  for attempt in 1..=RETRIES {
    match download_crx(url) {
      Ok(bytes) => return Ok(bytes),
      Err(error) => {
        if attempt >= RETRIES {
          return Err(anyhow!("download failed after retries: {error:#}"));
        }
        warn!(
          "[update] download failed (attempt {attempt}/{RETRIES}): {error:#}; retrying in {WAIT_SECS}s"
        );
        std::thread::sleep(std::time::Duration::from_secs(WAIT_SECS));
      }
    }
  }
  Err(anyhow!("download failed after retries"))
}

fn collect_user_extension_dirs(user_dir: &Path) -> Result<Vec<PathBuf>> {
  if is_extension_dir(user_dir) {
    return Ok(vec![user_dir.to_path_buf()]);
  }

  let mut dirs = Vec::new();
  if user_dir.is_dir() {
    for entry in fs::read_dir(user_dir)? {
      let path = entry?.path();
      if path.is_dir() && is_extension_dir(&path) {
        dirs.push(path);
      }
    }
  }

  Ok(dirs)
}

#[cfg(target_os = "windows")]
pub(crate) fn install_extensions_and_open(
  webview: PlatformWebview,
  line_dir: PathBuf,
  user_dir: PathBuf,
  entry_path: String,
) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  attach_new_window_handler(&webview)?;
  attach_permission_handler(&webview)?;
  unsafe {
    let settings = core.Settings()?;
    settings.SetIsScriptEnabled(true)?;
  }
  let profile = unsafe {
    core
      .cast::<ICoreWebView2_13>()?
      .Profile()?
      .cast::<ICoreWebView2Profile7>()?
  };

  let line_extension = add_browser_extension(&profile, &line_dir)?;
  ensure_extension_enabled(&line_extension)?;
  let line_id = browser_extension_id(&line_extension)?;

  for user_extension in collect_user_extension_dirs(&user_dir)? {
    let extension = add_browser_extension(&profile, &user_extension)?;
    ensure_extension_enabled(&extension)?;
  }

  let page_url = format!("chrome-extension://{line_id}{entry_path}");
  info!("[open] {}", page_url);
  let target = HSTRING::from(page_url.as_str());
  unsafe {
    core.Navigate(&target)?;
  }
  if let Err(error) = log_cookies_snapshot(&webview, "initial") {
    warn!("[cookie] initial failed: {error:#}");
  }

  Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn log_cookies_snapshot(webview: &PlatformWebview, tag: &str) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  log_all_cookies_summary(&core, tag)?;
  log_cookies(&core, tag, "https://access.line.me")?;
  log_cookies(&core, tag, "https://line.me")?;
  log_cookies(&core, tag, "https://api.line.me")?;
  Ok(())
}

#[cfg(target_os = "windows")]
fn log_cookies(core: &ICoreWebView2, tag: &str, uri: &str) -> Result<()> {
  let cookies = collect_cookies(core, Some(uri))?;
  let session_count = cookies.iter().filter(|cookie| cookie.is_session).count();
  debug!(
    "[cookie] {tag} {uri} count={} session={}",
    cookies.len(),
    session_count
  );
  Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn persist_session_cookies_snapshot(webview: &PlatformWebview, tag: &str) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  persist_all_session_cookies(&core, tag)?;
  persist_session_cookies(&core, tag, "https://access.line.me")?;
  persist_session_cookies(&core, tag, "https://line.me")?;
  persist_session_cookies(&core, tag, "https://api.line.me")?;
  Ok(())
}

#[cfg(target_os = "windows")]
fn persist_session_cookies(core: &ICoreWebView2, tag: &str, uri: &str) -> Result<()> {
  let mut cookies = collect_cookies(core, Some(uri))?;
  if cookies.is_empty() {
    debug!("[cookie] {tag} {uri} persist skipped (count=0)");
    return Ok(());
  }

  let session_total = cookies.iter().filter(|cookie| cookie.is_session).count();
  if session_total == 0 {
    debug!("[cookie] {tag} {uri} persist skipped (no session cookies)");
    return Ok(());
  }

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as f64;
  let expires = now + (365.0 * 24.0 * 60.0 * 60.0);

  let webview = core.cast::<ICoreWebView2_2>()?;
  let cookie_manager = unsafe { webview.CookieManager()? };
  let mut updated = 0usize;

  let host_fallback = Url::parse(uri)
    .ok()
    .and_then(|url| url.host_str().map(|host| host.to_string()))
    .unwrap_or_default();

  for cookie in cookies.iter_mut() {
    if !cookie.is_session {
      continue;
    }
    let domain = if cookie.domain.is_empty() {
      host_fallback.as_str()
    } else {
      cookie.domain.as_str()
    };
    if domain.is_empty() {
      continue;
    }
    let path = if cookie.path.is_empty() {
      "/"
    } else {
      cookie.path.as_str()
    };
    let name_hs = HSTRING::from(cookie.name.as_str());
    let value_hs = HSTRING::from(cookie.value.as_str());
    let domain_hs = HSTRING::from(domain);
    let path_hs = HSTRING::from(path);
    let new_cookie =
      unsafe { cookie_manager.CreateCookie(&name_hs, &value_hs, &domain_hs, &path_hs)? };
    unsafe {
      new_cookie.SetIsHttpOnly(cookie.is_http_only)?;
      new_cookie.SetIsSecure(cookie.is_secure)?;
      new_cookie.SetSameSite(cookie.same_site)?;
      new_cookie.SetExpires(expires)?;
      cookie_manager.AddOrUpdateCookie(&new_cookie)?;
    }
    updated += 1;
  }

  debug!("[cookie] {tag} {uri} persisted {updated}/{session_total} session cookies");
  Ok(())
}

#[cfg(target_os = "windows")]
fn collect_cookies(core: &ICoreWebView2, uri: Option<&str>) -> Result<Vec<CookieInfo>> {
  let webview = core.cast::<ICoreWebView2_2>()?;
  let cookie_manager = unsafe { webview.CookieManager()? };
  let (tx, rx) = mpsc::channel();
  unsafe {
    let handler = GetCookiesCompletedHandler::create(Box::new(move |result, cookies| {
      result?;
      let mut out: Vec<CookieInfo> = Vec::new();
      if let Some(cookies) = cookies {
        let mut count = 0;
        cookies.Count(&mut count)?;
        for idx in 0..count {
          let cookie = cookies.GetValueAtIndex(idx)?;
          let mut name_ptr = PWSTR::null();
          cookie.Name(&mut name_ptr)?;
          let name = take_pwstr(name_ptr);

          let mut value_ptr = PWSTR::null();
          cookie.Value(&mut value_ptr)?;
          let value = take_pwstr(value_ptr);

          let mut domain_ptr = PWSTR::null();
          cookie.Domain(&mut domain_ptr)?;
          let domain = take_pwstr(domain_ptr);

          let mut path_ptr = PWSTR::null();
          cookie.Path(&mut path_ptr)?;
          let path = take_pwstr(path_ptr);

          let mut is_session = BOOL(0);
          cookie.IsSession(&mut is_session)?;

          let mut is_http_only = BOOL(0);
          cookie.IsHttpOnly(&mut is_http_only)?;

          let mut is_secure = BOOL(0);
          cookie.IsSecure(&mut is_secure)?;

          let mut same_site = COREWEBVIEW2_COOKIE_SAME_SITE_KIND(0);
          cookie.SameSite(&mut same_site)?;

          out.push(CookieInfo {
            name,
            value,
            domain,
            path,
            is_session: is_session.as_bool(),
            is_http_only: is_http_only.as_bool(),
            is_secure: is_secure.as_bool(),
            same_site,
          });
        }
      }
      let _ = tx.send(out);
      Ok(())
    }));

    match uri {
      Some(uri) => {
        let uri_hs = HSTRING::from(uri);
        cookie_manager.GetCookies(&uri_hs, &handler)?;
      }
      None => {
        cookie_manager.GetCookies(PCWSTR::null(), &handler)?;
      }
    }
  }
  wait_with_pump(rx).map_err(Into::into)
}

#[cfg(target_os = "windows")]
struct CookieInfo {
  name: String,
  value: String,
  domain: String,
  path: String,
  is_session: bool,
  is_http_only: bool,
  is_secure: bool,
  same_site: COREWEBVIEW2_COOKIE_SAME_SITE_KIND,
}

#[cfg(target_os = "windows")]
fn log_all_cookies_summary(core: &ICoreWebView2, tag: &str) -> Result<()> {
  let cookies = collect_cookies(core, None)?;
  let session_count = cookies.iter().filter(|cookie| cookie.is_session).count();
  debug!(
    "[cookie] {tag} all count={} session={}",
    cookies.len(),
    session_count
  );
  Ok(())
}

#[cfg(target_os = "windows")]
fn persist_all_session_cookies(core: &ICoreWebView2, tag: &str) -> Result<()> {
  let mut cookies = collect_cookies(core, None)?;
  if cookies.is_empty() {
    debug!("[cookie] {tag} all persist skipped (count=0)");
    return Ok(());
  }
  let session_total = cookies.iter().filter(|cookie| cookie.is_session).count();
  if session_total == 0 {
    debug!("[cookie] {tag} all persist skipped (no session cookies)");
    return Ok(());
  }

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as f64;
  let expires = now + (365.0 * 24.0 * 60.0 * 60.0);

  let webview = core.cast::<ICoreWebView2_2>()?;
  let cookie_manager = unsafe { webview.CookieManager()? };
  let mut updated = 0usize;

  for cookie in cookies.iter_mut() {
    if !cookie.is_session {
      continue;
    }
    if cookie.domain.is_empty() {
      continue;
    }
    let path = if cookie.path.is_empty() {
      "/"
    } else {
      cookie.path.as_str()
    };
    let name_hs = HSTRING::from(cookie.name.as_str());
    let value_hs = HSTRING::from(cookie.value.as_str());
    let domain_hs = HSTRING::from(cookie.domain.as_str());
    let path_hs = HSTRING::from(path);
    let new_cookie =
      unsafe { cookie_manager.CreateCookie(&name_hs, &value_hs, &domain_hs, &path_hs)? };
    unsafe {
      new_cookie.SetIsHttpOnly(cookie.is_http_only)?;
      new_cookie.SetIsSecure(cookie.is_secure)?;
      new_cookie.SetSameSite(cookie.same_site)?;
      new_cookie.SetExpires(expires)?;
      cookie_manager.AddOrUpdateCookie(&new_cookie)?;
    }
    updated += 1;
  }

  debug!("[cookie] {tag} all persisted {updated}/{session_total} session cookies");
  Ok(())
}
