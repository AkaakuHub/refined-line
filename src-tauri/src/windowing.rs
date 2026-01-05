use std::sync::atomic::{AtomicUsize, Ordering};
use url::Url;

static NEXT_WINDOW_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) fn next_popup_label() -> String {
  let id = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed);
  format!("popup-{id}")
}

pub(crate) fn should_open_external(url: &Url) -> bool {
  match url.scheme() {
    "http" | "https" => !is_localhost_url(url),
    "mailto" | "tel" => true,
    _ => false,
  }
}

// Avoid tauri frontend URL opening in the browser.
fn is_localhost_url(url: &Url) -> bool {
  matches!(
    url.host_str(),
    Some("localhost") | Some("127.0.0.1") | Some("::1")
  )
}

#[cfg(target_os = "windows")]
use anyhow::Result;
#[cfg(target_os = "windows")]
use log::warn;
#[cfg(target_os = "windows")]
use tauri::webview::PlatformWebview;
#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use tauri_plugin_opener::OpenerExt;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
  COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS,
  COREWEBVIEW2_PERMISSION_STATE_ALLOW,
};
#[cfg(target_os = "windows")]
use webview2_com::{
  take_pwstr, NewWindowRequestedEventHandler, PermissionRequestedEventHandler,
  WindowCloseRequestedEventHandler,
};
#[cfg(target_os = "windows")]
use windows::core::PWSTR;

#[cfg(target_os = "windows")]
pub(crate) fn attach_new_window_handler(
  app_handle: tauri::AppHandle,
  webview: &PlatformWebview,
) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  let handler = NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
    let Some(args) = args else {
      return Ok(());
    };
    let mut uri_ptr = PWSTR::null();
    unsafe {
      args.Uri(&mut uri_ptr)?;
    }
    let uri = take_pwstr(uri_ptr);
    if let Ok(url) = Url::parse(&uri) {
      if should_open_external(&url) {
        if let Err(error) = app_handle.opener().open_url(url.as_str(), None::<&str>) {
          warn!("[open] failed: {error:#}");
        }
        unsafe {
          args.SetHandled(true)?;
        }
      }
    }
    Ok(())
  }));

  let mut token = 0i64;
  unsafe {
    core.add_NewWindowRequested(&handler, &mut token)?;
  }
  Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn attach_permission_handler(webview: &PlatformWebview) -> Result<()> {
  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  let handler = PermissionRequestedEventHandler::create(Box::new(move |_, args| {
    let Some(args) = args else {
      return Ok(());
    };
    let mut kind = COREWEBVIEW2_PERMISSION_KIND(0);
    unsafe {
      args.PermissionKind(&mut kind)?;
    }
    if kind == COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS {
      unsafe {
        args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
      }
    }
    Ok(())
  }));

  let mut token = 0i64;
  unsafe {
    core.add_PermissionRequested(&handler, &mut token)?;
  }
  Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn attach_close_requested_handler(
  app_handle: tauri::AppHandle,
  webview: &PlatformWebview,
  label: String,
) -> Result<()> {
  if !label.starts_with("popup-") {
    return Ok(());
  }

  let controller = webview.controller();
  let core = unsafe { controller.CoreWebView2()? };
  let handler = WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
    if let Some(window) = app_handle.get_webview_window(&label) {
      let _ = window.close();
    }
    Ok(())
  }));

  let mut token = 0i64;
  unsafe {
    core.add_WindowCloseRequested(&handler, &mut token)?;
  }
  Ok(())
}
