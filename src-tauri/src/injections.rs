const FONT_SCRIPT: &str = include_str!("../injections/font.js");
const NO_SCROLLBAR_SCRIPT: &str = include_str!("../injections/no-scrollbar.js");
const SIDEBAR_SCRIPT: &str = include_str!("../injections/sidebar.js");
const HOTKEYS_SCRIPT: &str = include_str!("../injections/hotkeys.js");
const TITLEBAR_SCRIPT: &str = include_str!("../injections/titlebar.js");

pub(crate) fn inject_scripts<R: tauri::Runtime>(
  webview: &tauri::Webview<R>,
) -> Result<(), tauri::Error> {
  webview.eval(FONT_SCRIPT)?;
  webview.eval(NO_SCROLLBAR_SCRIPT)?;
  webview.eval(SIDEBAR_SCRIPT)?;
  Ok(())
}

pub(crate) fn inject_titlebar<R: tauri::Runtime>(
  webview: &tauri::Webview<R>,
) -> Result<(), tauri::Error> {
  webview.eval(TITLEBAR_SCRIPT)?;
  Ok(())
}

pub(crate) fn inject_hotkeys<R: tauri::Runtime>(
  webview: &tauri::Webview<R>,
) -> Result<(), tauri::Error> {
  webview.eval(HOTKEYS_SCRIPT)?;
  Ok(())
}
