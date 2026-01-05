use log::{debug, info, warn};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

pub fn spawn_update_check(app: &AppHandle) {
  let app = app.clone();
  tauri::async_runtime::spawn(async move {
    let updater = match app.updater() {
      Ok(updater) => updater,
      Err(error) => {
        warn!("[updater] init failed: {error:#}");
        return;
      }
    };

    match updater.check().await {
      Ok(Some(update)) => {
        info!("[updater] update available: {}", update.version);
        prompt_update(app.clone(), update);
      }
      Ok(None) => debug!("[updater] no update"),
      Err(error) => warn!("[updater] check failed: {error:#}"),
    }
  });
}

fn prompt_update(app: AppHandle, update: Update) {
  let message = build_update_message(&update);
  app
    .dialog()
    .message(message)
    .title("更新があります")
    .kind(MessageDialogKind::Info)
    .buttons(MessageDialogButtons::OkCancelCustom(
      "更新する".into(),
      "あとで".into(),
    ))
    .show(move |confirmed| {
      if !confirmed {
        info!("[updater] user skipped update {}", update.version);
        return;
      }

      let app = app.clone();
      let update = update.clone();
      tauri::async_runtime::spawn(async move {
        info!("[updater] downloading {}", update.version);
        let result = update.download_and_install(
          |chunk, total| {
            debug!("[updater] downloaded {} bytes ({total:?})", chunk);
          },
          || {
            debug!("[updater] download finished");
          },
        );
        if let Err(error) = result.await {
          warn!("[updater] install failed: {error:#}");
          return;
        }

        info!("[updater] installed, restarting");
        app.restart();
      });
    });
}

fn build_update_message(update: &Update) -> String {
  let mut message = format!(
    "新しいバージョン {} が見つかりました。\n更新後にアプリは再起動されます。\n今すぐ更新しますか？",
    update.version
  );
  if let Some(body) = update.body.as_ref() {
    if !body.trim().is_empty() {
      message.push_str("\n\n更新内容:\n");
      message.push_str(body.trim());
    }
  }
  message
}
