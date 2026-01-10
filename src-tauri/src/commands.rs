use crate::settings::{load_settings, save_settings, AppSettings};
use tauri::Window;

#[tauri::command]
pub(crate) fn get_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, String> {
  load_settings(&app_handle).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_settings(
  app_handle: tauri::AppHandle,
  settings: AppSettings,
) -> Result<AppSettings, String> {
  save_settings(&app_handle, &settings).map_err(|error| error.to_string())?;
  Ok(settings)
}

#[tauri::command]
pub(crate) fn get_is_dev() -> bool {
  tauri::is_dev()
}

#[tauri::command]
pub(crate) fn get_is_maximized(window: Window) -> Result<bool, String> {
  window.is_maximized().map_err(|error| error.to_string())
}
