use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct AppSettings {
  pub(crate) auto_start: bool,
  pub(crate) start_minimized: bool,
  pub(crate) content_protection: bool,
  pub(crate) log_level: String,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      auto_start: false,
      start_minimized: false,
      content_protection: true,
      log_level: crate::logger::DEFAULT_LOG_LEVEL.to_string(),
    }
  }
}

pub(crate) fn load_settings(app: &tauri::AppHandle) -> Result<AppSettings> {
  let path = settings_path(app)?;
  let raw = match fs::read_to_string(&path) {
    Ok(raw) => raw,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(AppSettings::default()),
    Err(error) => return Err(error.into()),
  };
  let parsed = serde_json::from_str(&raw);
  Ok(parsed.unwrap_or_default())
}

pub(crate) fn save_settings(app: &tauri::AppHandle, settings: &AppSettings) -> Result<()> {
  let path = settings_path(app)?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let payload = serde_json::to_string_pretty(settings)?;
  fs::write(path, payload)?;
  Ok(())
}

pub(crate) fn update_content_protection(
  app: &tauri::AppHandle,
  enabled: bool,
) -> Result<AppSettings> {
  let mut settings = load_settings(app).unwrap_or_default();
  settings.content_protection = enabled;
  save_settings(app, &settings)?;
  Ok(settings)
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf> {
  let dir = app
    .path()
    .app_data_dir()
    .map_err(|error| anyhow!("settings dir error: {error}"))?;
  Ok(dir.join("settings.json"))
}
