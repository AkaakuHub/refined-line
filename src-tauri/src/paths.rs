use anyhow::{anyhow, Result};
use std::path::PathBuf;
pub(crate) fn app_data_root(app: &tauri::AppHandle) -> Result<PathBuf> {
  let app_name = app.package_info().name.clone();
  let base = dirs::data_dir().ok_or_else(|| anyhow!("app data dir error"))?;
  Ok(base.join(app_name))
}

pub(crate) fn profile_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
  Ok(app_data_root(app)?.join("webview2-profile"))
}

pub(crate) fn profile_reset_marker(app: &tauri::AppHandle) -> Result<PathBuf> {
  Ok(app_data_root(app)?.join("reset-profile.flag"))
}
