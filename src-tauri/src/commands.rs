use crate::paths::profile_reset_marker;
use crate::settings::{load_settings, save_settings, AppSettings};
use log::info;
use tauri::Window;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

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

#[tauri::command]
pub(crate) fn reset_profile(app_handle: tauri::AppHandle) -> Result<(), String> {
  let marker = profile_reset_marker(&app_handle).map_err(|error| error.to_string())?;
  if let Some(parent) = marker.parent() {
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
  }
  std::fs::write(&marker, "reset").map_err(|error| error.to_string())?;
  info!("[webview] reset profile requested");
  app_handle.exit(0);
  Ok(())
}

#[tauri::command]
pub(crate) async fn confirm_reset_profile(app_handle: tauri::AppHandle) -> Result<bool, String> {
  let (tx, mut rx) = tauri::async_runtime::channel(1);
  app_handle
    .dialog()
    .message(
      "次回起動時にWebView のプロファイルデータを削除します。ログイン情報やキャッシュが消えます。続行しますか？",
    )
    .title("プロファイルデータのリセット")
    .buttons(MessageDialogButtons::YesNo)
    .show(move |confirmed| {
      let _ = tx.try_send(confirmed);
    });
  rx.recv()
    .await
    .ok_or_else(|| "dialog cancelled".to_string())
}
