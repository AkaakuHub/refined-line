use crate::content_protection::{is_content_protected, set_content_protection_from_app};
use crate::logger::{apply_log_level, LogLevel};
use crate::settings::{load_settings, save_settings};
use crate::tray::set_tray_enabled;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, PredefinedMenuItem, Submenu};
use tauri::{is_dev, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

const MENU_CONTENT_PROTECTION_ID: &str = "menu.content_protection";
const MENU_AUTOSTART_ID: &str = "menu.autostart";
const MENU_START_MINIMIZED_ID: &str = "menu.start_minimized";
const MENU_LOG_ERROR_ID: &str = "menu.log.error";
const MENU_LOG_WARN_ID: &str = "menu.log.warn";
const MENU_LOG_INFO_ID: &str = "menu.log.info";
const MENU_LOG_DEBUG_ID: &str = "menu.log.debug";
const MENU_LOG_VERBOSE_ID: &str = "menu.log.verbose";

pub(crate) struct MenuState {
  pub(crate) menu: Menu<Wry>,
  content_protection: CheckMenuItem<Wry>,
  autostart: CheckMenuItem<Wry>,
  start_minimized: CheckMenuItem<Wry>,
  log_error: CheckMenuItem<Wry>,
  log_warn: CheckMenuItem<Wry>,
  log_info: CheckMenuItem<Wry>,
  log_debug: CheckMenuItem<Wry>,
  log_verbose: CheckMenuItem<Wry>,
}

pub(crate) fn build_menu(
  app_handle: &tauri::AppHandle,
  settings: &crate::settings::AppSettings,
) -> tauri::Result<MenuState> {
  let autostart_enabled = app_handle
    .autolaunch()
    .is_enabled()
    .unwrap_or(settings.auto_start);
  let effective_log_level = crate::logger::resolve_log_level(&settings.log_level);

  let content_protection = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_CONTENT_PROTECTION_ID),
    "画面を保護",
    true,
    settings.content_protection,
    Some("Alt+H"),
  )?;
  let autostart = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_AUTOSTART_ID),
    "Windows 起動時に自動起動",
    true,
    autostart_enabled,
    None::<&str>,
  )?;
  let start_minimized = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_START_MINIMIZED_ID),
    "起動時に最小化",
    true,
    settings.start_minimized,
    None::<&str>,
  )?;
  let log_error = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_LOG_ERROR_ID),
    "Error",
    true,
    effective_log_level == LogLevel::Error,
    None::<&str>,
  )?;
  let log_warn = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_LOG_WARN_ID),
    "Warn",
    true,
    effective_log_level == LogLevel::Warn,
    None::<&str>,
  )?;
  let log_info = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_LOG_INFO_ID),
    "Info",
    true,
    effective_log_level == LogLevel::Info,
    None::<&str>,
  )?;
  let log_debug = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_LOG_DEBUG_ID),
    "Debug",
    true,
    effective_log_level == LogLevel::Debug,
    None::<&str>,
  )?;
  let log_verbose = CheckMenuItem::with_id(
    app_handle,
    MenuId::new(MENU_LOG_VERBOSE_ID),
    "Verbose",
    true,
    effective_log_level == LogLevel::Verbose,
    None::<&str>,
  )?;

  let log_menu = Submenu::with_items(
    app_handle,
    "ログレベル",
    true,
    &[&log_error, &log_warn, &log_info, &log_debug, &log_verbose],
  )?;

  let settings_separator = PredefinedMenuItem::separator(app_handle)?;
  let dev_separator = if is_dev() {
    Some(PredefinedMenuItem::separator(app_handle)?)
  } else {
    None
  };
  let close_item = PredefinedMenuItem::close_window(app_handle, None)?;

  let mut settings_items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![
    &content_protection,
    &autostart,
    &start_minimized,
    &settings_separator,
  ];
  if is_dev() {
    settings_items.push(&log_menu);
    if let Some(separator) = dev_separator.as_ref() {
      settings_items.push(separator);
    }
  }
  settings_items.push(&close_item);
  let settings_menu = Submenu::with_items(app_handle, "設定", true, &settings_items)?;

  let menu = Menu::with_items(app_handle, &[&settings_menu])?;
  Ok(MenuState {
    menu,
    content_protection,
    autostart,
    start_minimized,
    log_error,
    log_warn,
    log_info,
    log_debug,
    log_verbose,
  })
}

pub(crate) fn handle_menu_event(app_handle: &tauri::AppHandle, event: MenuEvent) {
  match event.id() {
    id if id == MENU_CONTENT_PROTECTION_ID => {
      let target = !is_content_protected(app_handle);
      if let Ok(enabled) = set_content_protection_from_app(app_handle, target) {
        set_menu_checked(app_handle, MENU_CONTENT_PROTECTION_ID, enabled);
      }
    }
    id if id == MENU_AUTOSTART_ID => {
      let autolaunch = app_handle.autolaunch();
      let fallback_enabled = load_settings(app_handle)
        .map(|settings| settings.auto_start)
        .unwrap_or(false);
      let current = autolaunch.is_enabled().unwrap_or(fallback_enabled);
      let target = !current;
      let result = if target {
        autolaunch.enable()
      } else {
        autolaunch.disable()
      };
      if result.is_err() {
        let enabled = autolaunch.is_enabled().unwrap_or(false);
        set_menu_checked(app_handle, MENU_AUTOSTART_ID, enabled);
        return;
      }
      if let Ok(mut settings) = load_settings(app_handle) {
        settings.auto_start = target;
        let _ = save_settings(app_handle, &settings);
      }
      let enabled = autolaunch.is_enabled().unwrap_or(target);
      set_menu_checked(app_handle, MENU_AUTOSTART_ID, enabled);
    }
    id if id == MENU_START_MINIMIZED_ID => {
      let current = load_settings(app_handle)
        .map(|settings| settings.start_minimized)
        .unwrap_or(false);
      let target = !current;
      let tray_enabled = set_tray_enabled(app_handle, target);
      if let Ok(mut settings) = load_settings(app_handle) {
        settings.start_minimized = tray_enabled;
        let _ = save_settings(app_handle, &settings);
      }
      set_menu_checked(app_handle, MENU_START_MINIMIZED_ID, tray_enabled);
    }
    id if id == MENU_LOG_ERROR_ID => {
      update_log_level(app_handle, LogLevel::Error);
    }
    id if id == MENU_LOG_WARN_ID => {
      update_log_level(app_handle, LogLevel::Warn);
    }
    id if id == MENU_LOG_INFO_ID => {
      update_log_level(app_handle, LogLevel::Info);
    }
    id if id == MENU_LOG_DEBUG_ID => {
      update_log_level(app_handle, LogLevel::Debug);
    }
    id if id == MENU_LOG_VERBOSE_ID => {
      update_log_level(app_handle, LogLevel::Verbose);
    }
    _ => {}
  }
}

pub(crate) fn set_menu_checked(app_handle: &tauri::AppHandle, id: &str, checked: bool) {
  let Some(state) = app_handle.try_state::<MenuState>() else {
    return;
  };
  match id {
    MENU_CONTENT_PROTECTION_ID => {
      let _ = state.content_protection.set_checked(checked);
    }
    MENU_AUTOSTART_ID => {
      let _ = state.autostart.set_checked(checked);
    }
    MENU_START_MINIMIZED_ID => {
      let _ = state.start_minimized.set_checked(checked);
    }
    MENU_LOG_ERROR_ID => {
      let _ = state.log_error.set_checked(checked);
    }
    MENU_LOG_WARN_ID => {
      let _ = state.log_warn.set_checked(checked);
    }
    MENU_LOG_INFO_ID => {
      let _ = state.log_info.set_checked(checked);
    }
    MENU_LOG_DEBUG_ID => {
      let _ = state.log_debug.set_checked(checked);
    }
    MENU_LOG_VERBOSE_ID => {
      let _ = state.log_verbose.set_checked(checked);
    }
    _ => {}
  }
}

fn update_log_level(app_handle: &tauri::AppHandle, level: LogLevel) {
  apply_log_level(level);
  if let Ok(mut settings) = load_settings(app_handle) {
    settings.log_level = level.as_str().to_string();
    let _ = save_settings(app_handle, &settings);
  }
  set_menu_checked(app_handle, MENU_LOG_ERROR_ID, level == LogLevel::Error);
  set_menu_checked(app_handle, MENU_LOG_WARN_ID, level == LogLevel::Warn);
  set_menu_checked(app_handle, MENU_LOG_INFO_ID, level == LogLevel::Info);
  set_menu_checked(app_handle, MENU_LOG_DEBUG_ID, level == LogLevel::Debug);
  set_menu_checked(app_handle, MENU_LOG_VERBOSE_ID, level == LogLevel::Verbose);
}

pub(crate) fn menu_content_protection_id() -> &'static str {
  MENU_CONTENT_PROTECTION_ID
}
