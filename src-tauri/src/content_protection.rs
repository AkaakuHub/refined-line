use crate::settings::update_content_protection;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Mutex,
};
use tauri::{Emitter, Manager, Runtime, State};

const HIDDEN_TITLE_SUFFIX: &str = " - hidden window";

pub(crate) struct WindowState {
  protected: AtomicBool,
  titles: Mutex<HashMap<String, String>>,
}

impl WindowState {
  pub(crate) fn new(protected: bool) -> Self {
    Self {
      protected: AtomicBool::new(protected),
      titles: Mutex::new(HashMap::new()),
    }
  }
}

pub(crate) fn is_content_protected<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> bool {
  app_handle
    .state::<WindowState>()
    .protected
    .load(Ordering::Relaxed)
}

pub(crate) fn set_content_protection_from_app(
  app_handle: &tauri::AppHandle,
  enabled: bool,
) -> Result<bool, String> {
  let state = app_handle.state::<WindowState>();
  set_content_protection_state(app_handle, &state, enabled)
}

fn apply_content_protection<R: Runtime>(
  app_handle: &tauri::AppHandle<R>,
  protected: bool,
) -> usize {
  let mut count = 0usize;
  for (label, window) in app_handle.webview_windows() {
    count += 1;
    let base_title = get_base_title(app_handle, &label);
    set_content_protected(&window, &label, protected, base_title.as_deref());
  }
  count
}

#[tauri::command]
pub(crate) fn toggle_content_protection(
  app_handle: tauri::AppHandle,
  state: State<WindowState>,
) -> Result<bool, String> {
  let enabled = !state.protected.load(Ordering::Relaxed);
  set_content_protection_state(&app_handle, &state, enabled)
}

#[tauri::command]
pub(crate) fn get_content_protection(state: State<WindowState>) -> Result<bool, String> {
  Ok(state.protected.load(Ordering::Relaxed))
}

#[tauri::command]
pub(crate) fn set_content_protection(
  app_handle: tauri::AppHandle,
  state: State<WindowState>,
  enabled: bool,
) -> Result<bool, String> {
  set_content_protection_state(&app_handle, &state, enabled)
}

fn set_content_protection_state(
  app_handle: &tauri::AppHandle,
  state: &State<WindowState>,
  enabled: bool,
) -> Result<bool, String> {
  state.protected.store(enabled, Ordering::Relaxed);
  let count = apply_content_protection(app_handle, enabled);
  if let Err(error) = update_content_protection(app_handle, enabled) {
    warn!("[content-protected] save failed: {error:#}");
  }
  crate::app_menu::set_menu_checked(
    app_handle,
    crate::app_menu::menu_content_protection_id(),
    enabled,
  );
  let _ = app_handle.emit("content-protection-changed", enabled);
  info!("[content-protected] set {enabled} windows={count}");
  Ok(enabled)
}

pub(crate) fn set_content_protected<R: Runtime>(
  window: &tauri::WebviewWindow<R>,
  label: &str,
  protected: bool,
  base_title: Option<&str>,
) {
  if let Err(error) = window.set_content_protected(protected) {
    warn!("[content-protected] {label} failed: {error:#}");
  } else {
    if let Some(base_title) = base_title {
      update_window_title(window, base_title, protected);
    }
    debug!("[content-protected] {label} set {protected}");
  }
}

fn update_window_title<R: Runtime>(
  window: &tauri::WebviewWindow<R>,
  base_title: &str,
  protected: bool,
) {
  let next = if protected {
    format!("{base_title}{HIDDEN_TITLE_SUFFIX}")
  } else {
    base_title.to_string()
  };
  let _ = window.set_title(&next);
}

pub(crate) fn store_base_title<R: Runtime>(
  app_handle: &tauri::AppHandle<R>,
  label: &str,
  base_title: &str,
) {
  if let Ok(mut titles) = app_handle.state::<WindowState>().titles.lock() {
    titles.insert(label.to_string(), base_title.to_string());
  }
}

fn get_base_title<R: Runtime>(app_handle: &tauri::AppHandle<R>, label: &str) -> Option<String> {
  app_handle
    .state::<WindowState>()
    .titles
    .lock()
    .ok()
    .and_then(|titles| titles.get(label).cloned())
}

pub(crate) fn ensure_base_title<R: Runtime>(
  app_handle: &tauri::AppHandle<R>,
  window: &tauri::WebviewWindow<R>,
  label: &str,
) -> String {
  if let Some(existing) = get_base_title(app_handle, label) {
    return existing;
  }
  let title = window.title().unwrap_or_else(|_| label.to_string());
  store_base_title(app_handle, label, &title);
  title
}
