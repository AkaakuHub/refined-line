use anyhow::{anyhow, Result};
use log::{info, warn};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::config::load_config;
use crate::crx::{
  build_update_url, check_update, download_crx, ensure_clean_dir, extract_zip, inject_manifest_key,
  parse_crx3, UpdateCheck,
};

#[cfg(target_os = "windows")]
use crate::windowing::{attach_new_window_handler, attach_permission_handler};
#[cfg(target_os = "windows")]
use std::sync::mpsc;
#[cfg(target_os = "windows")]
use tauri::webview::PlatformWebview;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
  ICoreWebView2BrowserExtension, ICoreWebView2Profile7, ICoreWebView2_13,
};
#[cfg(target_os = "windows")]
use webview2_com::{
  take_pwstr, wait_with_pump, BrowserExtensionEnableCompletedHandler,
  ProfileAddBrowserExtensionCompletedHandler,
};
#[cfg(target_os = "windows")]
use windows::core::BOOL;
#[cfg(target_os = "windows")]
use windows::core::{Interface, HSTRING, PWSTR};

pub(crate) struct ExtensionSetup {
  pub(crate) line_dir: PathBuf,
  pub(crate) user_dir: PathBuf,
  pub(crate) updated: bool,
  pub(crate) update_failed: bool,
}

pub(crate) fn prepare_extensions(app: &tauri::AppHandle) -> Result<ExtensionSetup> {
  let config = load_config(app)?;
  let app_name = app.package_info().name.clone();
  let app_data = dirs::data_dir()
    .map(|dir| dir.join(&app_name))
    .or_else(|| {
      app
        .path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join(&app_name))
    })
    .ok_or_else(|| anyhow!("app data dir error"))?;

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

  Ok(())
}
