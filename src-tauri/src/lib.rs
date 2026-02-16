mod app_menu;
mod commands;
mod config;
mod content_protection;
mod crx;
mod extensions;
mod injections;
mod logger;
mod notify_badge;
mod paths;
mod settings;
mod tray;
mod updater;
mod windowing;

use app_menu::{build_menu, handle_menu_event, menu_action};
use commands::{
  confirm_reset_profile, get_is_dev, get_is_maximized, get_settings, reset_profile, update_settings,
};
use config::load_config;
use content_protection::{
  ensure_base_title, get_content_protection, is_content_protected, set_content_protected,
  set_content_protection, store_base_title, toggle_content_protection, WindowState,
};
#[cfg(target_os = "windows")]
use extensions::install_extensions_and_open;
use extensions::{
  log_cookies_snapshot, persist_session_cookies_snapshot, prepare_extensions, ExtensionSetup,
};
use injections::{inject_hotkeys, inject_scripts, inject_titlebar};
use log::{debug, error, info, warn};
use logger::{apply_log_level, build_plugin, resolve_log_level};
use notify_badge::{init_notify_badge_state, update_notification_badge};
use paths::{profile_dir, profile_reset_marker};
use settings::{load_settings, save_settings};
use tauri::webview::PageLoadEvent;
#[cfg(target_os = "windows")]
use tauri::webview::ScrollBarStyle;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tray::{init_tray_state, is_tray_enabled};
use updater::spawn_update_check;
#[cfg(target_os = "windows")]
use windowing::{
  attach_close_requested_handler, attach_new_window_handler, attach_permission_handler,
};
use windowing::{next_popup_label, should_open_external};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .on_page_load(|webview, payload| {
      if payload.event() != PageLoadEvent::Finished {
        return;
      }
      // let current_url = payload.url().as_str();
      let window = webview.window();
      let label: String = window.label().to_string();
      let _ = inject_hotkeys(webview);
      // if current_url.starts_with("chrome-extension://") {
      /*
      * 拡張機能内のHTMLファイル一覧（4種類）：
      index.html
      popup.html
      ltsmSandbox.html
      cropperSandbox.html
      */
      let _ = inject_scripts(webview);
      let _ = inject_titlebar(webview);
      // }
      let app_handle = window.app_handle().clone();
      let protected = is_content_protected(&app_handle);
      if let Some(webview_window) = app_handle.get_webview_window(&label) {
        let base_title = ensure_base_title(&app_handle, &webview_window, &label);
        set_content_protected(
          &webview_window,
          &label,
          protected,
          Some(base_title.as_str()),
        );
      }
    })
    .invoke_handler(tauri::generate_handler![
      toggle_content_protection,
      get_content_protection,
      set_content_protection,
      get_settings,
      update_settings,
      get_is_dev,
      get_is_maximized,
      confirm_reset_profile,
      reset_profile,
      update_notification_badge,
      menu_action
    ])
    .on_window_event(|window, event| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" && is_tray_enabled(window.app_handle()) {
          api.prevent_close();
          let _ = window.hide();
        }
      }
    })
    .plugin(build_plugin())
    .plugin(tauri_plugin_autostart::init(
      tauri_plugin_autostart::MacosLauncher::LaunchAgent,
      None,
    ))
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .setup(|app| {
      let app_handle = app.handle().clone();
      let mut settings = load_settings(&app_handle).unwrap_or_default();
      if let Ok(enabled) = app_handle.autolaunch().is_enabled() {
        if settings.auto_start != enabled {
          settings.auto_start = enabled;
          let _ = save_settings(&app_handle, &settings);
        }
      }
      app.manage(WindowState::new(settings.content_protection));
      init_notify_badge_state(&app_handle);
      apply_log_level(resolve_log_level(&settings.log_level));
      let config = load_config(&app_handle)?;
      let menu_state = build_menu(&app_handle, &settings)?;
      let profile_dir = profile_dir(&app_handle)?;
      let reset_marker = profile_reset_marker(&app_handle)?;
      if reset_marker.is_file() {
        info!("[webview] reset profile requested");
        if profile_dir.exists() {
          match std::fs::remove_dir_all(&profile_dir) {
            Ok(()) => {
              info!("[webview] profile dir removed");
              let _ = std::fs::remove_file(&reset_marker);
            }
            Err(error) => {
              warn!("[webview] profile reset failed: {error:#}");
            }
          }
        } else {
          let _ = std::fs::remove_file(&reset_marker);
        }
      }
      std::fs::create_dir_all(&profile_dir)?;

      let base_title = "refined-line";
      let conf = app_handle
        .config()
        .app
        .windows
        .get(0)
        .ok_or("window config not found")?;
      let mut builder = WebviewWindowBuilder::from_config(&app_handle, conf)?
        .data_directory(profile_dir.clone())
        .browser_extensions_enabled(true)
        .on_menu_event(|window, event| {
          handle_menu_event(window.app_handle(), event);
        })
        .on_navigation({
          let app_handle = app_handle.clone();
          move |url| {
            debug!("[open] on_navigation url={}", url);
            if should_open_external(url) {
              debug!("[open] on_navigation external url={}", url);
              let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
              return false;
            }
            true
          }
        })
        .on_new_window({
          let app_handle = app_handle.clone();
          let profile_dir = profile_dir.clone();
          move |url, features| {
            debug!("[open] on_new_window url={} features={:?}", url, features);
            if should_open_external(&url) {
              debug!("[open] on_new_window external url={}", url);
              let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
              return tauri::webview::NewWindowResponse::Deny;
            }

            let label = next_popup_label();
            let popup_label = label.clone();
            let popup_base_title = url.as_str().to_string();
            store_base_title(&app_handle, popup_label.as_str(), &popup_base_title);

            let mut builder =
              WebviewWindowBuilder::new(&app_handle, label, WebviewUrl::External(url.clone()))
                .data_directory(profile_dir.clone())
                .disable_drag_drop_handler()
                .title(popup_base_title.as_str())
                .decorations(false)
                .browser_extensions_enabled(true)
                .on_navigation({
                  let app_handle = app_handle.clone();
                  move |url| {
                    debug!("[open] popup on_navigation url={}", url);
                    if should_open_external(url) {
                      debug!("[open] popup on_navigation external url={}", url);
                      let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                      return false;
                    }
                    true
                  }
                });

            #[cfg(target_os = "windows")]
            {
              builder = builder.scroll_bar_style(ScrollBarStyle::FluentOverlay);
            }

            if let Some(size) = features.size() {
              builder = builder.inner_size(size.width, size.height);
            }

            #[cfg(windows)]
            {
              builder = builder.with_environment(features.opener().environment.clone());
            }

            let window = match builder.build() {
              Ok(window) => window,
              Err(error) => {
                error!("[new-window] failed: {error:#}");
                return tauri::webview::NewWindowResponse::Deny;
              }
            };

            let protected = is_content_protected(&app_handle);
            let window_for_tasks = window.clone();
            let app_handle_for_tasks = app_handle.clone();
            let popup_label_for_tasks = popup_label.clone();
            let popup_title_for_tasks = popup_base_title.clone();
            let _ = window.run_on_main_thread(move || {
              set_content_protected(
                &window_for_tasks,
                &popup_label_for_tasks,
                protected,
                Some(popup_title_for_tasks.as_str()),
              );

              #[cfg(target_os = "windows")]
              if let Err(error) = window_for_tasks.with_webview({
                let app_handle = app_handle_for_tasks.clone();
                let popup_label = popup_label_for_tasks.clone();
                move |webview| {
                  if let Err(error) = attach_new_window_handler(&webview) {
                    warn!("[new-window] handler failed: {error:#}");
                  }
                  if let Err(error) = attach_permission_handler(&webview) {
                    warn!("[new-window] permission handler failed: {error:#}");
                  }
                  if let Err(error) = attach_close_requested_handler(
                    app_handle.clone(),
                    &webview,
                    popup_label.clone(),
                  ) {
                    warn!("[new-window] close handler failed: {error:#}");
                  }
                }
              }) {
                error!("[new-window] with_webview failed: {error:#}");
              }
            });

            tauri::webview::NewWindowResponse::Create { window }
          }
        });

      #[cfg(target_os = "windows")]
      {
        builder = builder.scroll_bar_style(ScrollBarStyle::FluentOverlay);
      }

      let _window = builder.build()?;

      store_base_title(&app_handle, "main", base_title);
      app.manage(menu_state);
      if let Err(error) = init_tray_state(&app_handle, settings.start_minimized) {
        warn!("[tray] failed: {error:#}");
      }
      spawn_update_check(&app_handle);
      if settings.start_minimized {
        let _ = _window.minimize();
      }

      let entry_path = config.line_entry_path.clone();
      let app_handle_for_update = app_handle.clone();
      #[cfg(target_os = "windows")]
      std::thread::spawn(move || {
        let ExtensionSetup {
          line_dir,
          user_dir,
          updated,
          update_failed,
        } = match prepare_extensions(&app_handle_for_update) {
          Ok(result) => result,
          Err(error) => {
            error!("[update] failed: {error:#}");
            let app_handle = app_handle_for_update.clone();
            let dialog_handle = app_handle.clone();
            let _ = app_handle.run_on_main_thread(move || {
              dialog_handle
                .dialog()
                .message("アップデートに失敗しました。")
                .title("更新失敗")
                .show(|_| {});
            });
            return;
          }
        };

        let app_handle_for_install = app_handle_for_update.clone();
        let entry_path_for_install = entry_path.clone();
        let handle_for_task = app_handle_for_install.clone();
        let updated_for_dialog = updated;
        let update_failed_for_dialog = update_failed;
        let _ = app_handle_for_install.run_on_main_thread(move || {
          let Some(window) = handle_for_task.get_webview_window("main") else {
            warn!("[open] main window not found");
            return;
          };
          let line_dir_for_install = line_dir.clone();
          let user_dir_for_install = user_dir.clone();
          let entry_path_for_install = entry_path_for_install.clone();
          if let Err(error) = window.with_webview(move |webview| {
            let result = install_extensions_and_open(
              webview,
              line_dir_for_install.clone(),
              user_dir_for_install.clone(),
              entry_path_for_install.clone(),
            );
            if let Err(error) = result {
              error!("[open] failed: {error:#}");
              panic!("failed to open LINE extension");
            }
          }) {
            error!("[open] with_webview failed: {error:#}");
          }

          let handle_for_cookies = handle_for_task.clone();
          std::thread::spawn(move || {
            let schedule = [(10u64, "after_10s"), (30u64, "after_30s")];
            for (delay, tag) in schedule {
              std::thread::sleep(std::time::Duration::from_secs(delay));
              let handle = handle_for_cookies.clone();
              let tag = tag.to_string();
              let handle_for_main = handle.clone();
              let _ = handle.run_on_main_thread(move || {
                if let Some(window) = handle_for_main.get_webview_window("main") {
                  let tag_for_webview = tag.clone();
                  if let Err(error) = window.with_webview(move |webview| {
                    if let Err(error) = persist_session_cookies_snapshot(&webview, &tag_for_webview)
                    {
                      warn!("[cookie] {tag_for_webview} persist failed: {error:#}");
                    }
                    if let Err(error) = log_cookies_snapshot(&webview, &tag_for_webview) {
                      warn!("[cookie] {tag_for_webview} failed: {error:#}");
                    }
                  }) {
                    warn!("[cookie] {tag} with_webview failed: {error:#}");
                  }
                }
              });
            }
          });

          if update_failed_for_dialog {
            let app_handle = handle_for_task.clone();
            app_handle
              .dialog()
              .message("アップデートに失敗しました。")
              .title("更新失敗")
              .show(|_| {});
          } else if updated_for_dialog {
            let app_handle = handle_for_task.clone();
            app_handle
              .dialog()
              .message("拡張機能を更新しました。再起動しますか？")
              .title("更新完了")
              .show(move |confirmed| {
                if confirmed {
                  app_handle.restart();
                }
              });
          }
        });
      });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
