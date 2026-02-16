mod assets;

use anyhow::Result;
use assets::{badge_png_bytes, parse_badge_token, BadgeToken, BADGE_SIZES};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::image::Image;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Default)]
pub(crate) struct NotifyBadgeState {
  cache: Mutex<HashMap<(BadgeToken, u32), Image<'static>>>,
  last_applied: Mutex<Option<(Option<BadgeToken>, u32)>>,
}

pub(crate) fn init_notify_badge_state(app_handle: &tauri::AppHandle) {
  app_handle.manage(NotifyBadgeState::default());
}

#[tauri::command]
pub(crate) fn update_notification_badge(
  app_handle: tauri::AppHandle,
  text: Option<String>,
) -> Result<(), String> {
  #[cfg(target_os = "windows")]
  {
    apply_notification_badge(&app_handle, text.as_deref()).map_err(|error| error.to_string())
  }

  #[cfg(not(target_os = "windows"))]
  {
    let _ = app_handle;
    let _ = text;
    Ok(())
  }
}

#[cfg(target_os = "windows")]
fn apply_notification_badge<R: Runtime>(
  app_handle: &AppHandle<R>,
  text: Option<&str>,
) -> Result<()> {
  let Some(window) = app_handle.get_webview_window("main") else {
    return Ok(());
  };

  let token = parse_badge_token(text);
  let size = select_overlay_size(&window);
  let state = app_handle.state::<NotifyBadgeState>();
  let next = (token, size);

  {
    let mut last_applied = state
      .last_applied
      .lock()
      .map_err(|_| anyhow::anyhow!("notify badge state lock failed"))?;
    if last_applied.as_ref() == Some(&next) {
      return Ok(());
    }
    *last_applied = Some(next);
  }

  if let Some(token) = token {
    let image = get_cached_image(state.inner(), token, size)?;
    window.set_overlay_icon(Some(image))?;
  } else {
    window.set_overlay_icon(None)?;
  }
  Ok(())
}

#[cfg(target_os = "windows")]
fn select_overlay_size<R: Runtime>(window: &tauri::WebviewWindow<R>) -> u32 {
  const OVERLAY_BASE_SIZE: f64 = 20.0;
  let scale = window.scale_factor().unwrap_or(1.0_f64);
  let target = (OVERLAY_BASE_SIZE * scale).round().max(16.0_f64) as u32;
  BADGE_SIZES
    .iter()
    .copied()
    .min_by_key(|size| target.abs_diff(*size))
    .unwrap_or(32)
}

#[cfg(target_os = "windows")]
fn get_cached_image(
  state: &NotifyBadgeState,
  token: BadgeToken,
  size: u32,
) -> Result<Image<'static>> {
  let key = (token, size);
  let mut cache = state
    .cache
    .lock()
    .map_err(|_| anyhow::anyhow!("notify badge cache lock failed"))?;

  if let Some(image) = cache.get(&key) {
    return Ok(image.clone());
  }

  let bytes = badge_png_bytes(token, size)
    .ok_or_else(|| anyhow::anyhow!("notify badge asset not found size={size} token={token:?}"))?;
  let image = Image::from_bytes(bytes)?.to_owned();
  cache.insert(key, image.clone());
  Ok(image)
}
