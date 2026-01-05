use anyhow::Result;
use log::warn;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuId, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

const TRAY_QUIT_ID: &str = "tray.quit";

pub(crate) struct TrayState {
  enabled: bool,
  icon: Option<TrayIcon>,
}

pub(crate) fn init_tray_state(app: &tauri::AppHandle, enabled: bool) -> Result<()> {
  let icon = if enabled {
    Some(build_tray(app)?)
  } else {
    None
  };
  app.manage(Mutex::new(TrayState { enabled, icon }));
  Ok(())
}

pub(crate) fn set_tray_enabled(app: &tauri::AppHandle, enabled: bool) -> bool {
  let Some(state) = app.try_state::<Mutex<TrayState>>() else {
    return enabled;
  };
  let mut state = state.lock().expect("tray state lock");
  if state.enabled == enabled {
    return state.enabled;
  }

  if enabled {
    match build_tray(app) {
      Ok(icon) => {
        state.icon = Some(icon);
        state.enabled = true;
      }
      Err(error) => {
        warn!("[tray] enable failed: {error:#}");
        state.icon = None;
        state.enabled = false;
      }
    }
  } else {
    state.icon = None;
    state.enabled = false;
  }
  state.enabled
}

pub(crate) fn is_tray_enabled(app: &tauri::AppHandle) -> bool {
  let Some(state) = app.try_state::<Mutex<TrayState>>() else {
    return false;
  };
  state.lock().map(|state| state.enabled).unwrap_or(false)
}

fn build_tray(app: &tauri::AppHandle) -> Result<TrayIcon> {
  let quit = MenuItem::with_id(app, MenuId::new(TRAY_QUIT_ID), "閉じる", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&quit])?;

  let mut builder = TrayIconBuilder::new()
    .menu(&menu)
    .show_menu_on_left_click(false)
    .on_menu_event(|app, event| {
      if event.id() == TRAY_QUIT_ID {
        app.exit(0);
      }
    })
    .on_tray_icon_event(|tray, event| {
      if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
      } = event
      {
        let app_handle = tray.app_handle();
        if let Some(window) = app_handle.get_webview_window("main") {
          let _ = window.unminimize();
          let _ = window.show();
          let _ = window.set_focus();
        }
      }
    });

  if let Some(icon) = app.default_window_icon() {
    builder = builder.icon(icon.clone());
  }

  Ok(builder.build(app)?)
}
