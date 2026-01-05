use log::LevelFilter;
use std::env;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

pub(crate) const DEFAULT_LOG_LEVEL: &str = "info";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LogLevel {
  Error,
  Warn,
  Info,
  Debug,
  Verbose,
}

impl LogLevel {
  pub(crate) fn as_str(self) -> &'static str {
    match self {
      LogLevel::Error => "error",
      LogLevel::Warn => "warn",
      LogLevel::Info => "info",
      LogLevel::Debug => "debug",
      LogLevel::Verbose => "verbose",
    }
  }

  pub(crate) fn from_str(value: &str) -> Option<Self> {
    match value.trim().to_lowercase().as_str() {
      "error" => Some(LogLevel::Error),
      "warn" | "warning" => Some(LogLevel::Warn),
      "info" => Some(LogLevel::Info),
      "debug" => Some(LogLevel::Debug),
      "verbose" | "trace" => Some(LogLevel::Verbose),
      _ => None,
    }
  }

  pub(crate) fn to_level_filter(self) -> LevelFilter {
    match self {
      LogLevel::Error => LevelFilter::Error,
      LogLevel::Warn => LevelFilter::Warn,
      LogLevel::Info => LevelFilter::Info,
      LogLevel::Debug => LevelFilter::Debug,
      LogLevel::Verbose => LevelFilter::Trace,
    }
  }
}

pub(crate) fn resolve_log_level(settings_level: &str) -> LogLevel {
  if let Ok(env_level) = env::var("REFINED_LINE_LOG") {
    if let Some(parsed) = LogLevel::from_str(&env_level) {
      return parsed;
    }
  }
  LogLevel::from_str(settings_level).unwrap_or(LogLevel::Info)
}

pub(crate) fn apply_log_level(level: LogLevel) {
  log::set_max_level(level.to_level_filter());
}

pub(crate) fn build_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
  tauri_plugin_log::Builder::new()
    // Allow all records through the logger; runtime level is controlled via `log::set_max_level`.
    .level(LevelFilter::Trace)
    .timezone_strategy(TimezoneStrategy::UseLocal)
    .rotation_strategy(RotationStrategy::KeepAll)
    .target(Target::new(TargetKind::Stdout))
    .target(Target::new(TargetKind::LogDir { file_name: None }))
    .build()
}
