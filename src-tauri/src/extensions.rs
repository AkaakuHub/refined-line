use anyhow::{anyhow, Result};
use log::info;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::config::load_config;
use crate::crx::{
  build_update_url, download_crx, ensure_clean_dir, extract_zip, inject_manifest_key, parse_crx3,
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

pub(crate) fn prepare_extensions(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf)> {
  let config = load_config(app)?;
  let app_data = app
    .path()
    .app_data_dir()
    .map_err(|error| anyhow!("app data dir error: {error}"))?;

  let extensions_root = app_data.join("extensions");
  let line_dir = extensions_root.join("line");
  let user_dir = extensions_root.join("user");

  fs::create_dir_all(&user_dir)?;

  let update_url = build_update_url(&config.update2_base_url, &config.line_extension_id);
  info!("[update] start {}", update_url);
  let crx_bytes = download_crx(&update_url)?;
  let parsed = parse_crx3(&crx_bytes)?;

  ensure_clean_dir(&line_dir)?;
  extract_zip(&parsed.zip_bytes, &line_dir)?;
  inject_manifest_key(&line_dir, &parsed.public_key)?;

  Ok((line_dir, user_dir))
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
  app_handle: tauri::AppHandle,
  webview: PlatformWebview,
  line_dir: PathBuf,
  user_dir: PathBuf,
  entry_path: String,
) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  attach_new_window_handler(app_handle, &webview)?;
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
