//! Logging utilities (internal use).
//!
//! Provides a single helper (`init_from_dir`) that installs a `tracing` subscriber
//! based on an optional `logging.yml` file. This is invoked automatically by the
//! top-level builder; applications normally do not call it directly.
//!
//! # Configuration File: `logging.yml` (optional)
//! Keys:
//! * `filter` – full tracing filter expression (takes precedence over `level`)
//! * `level`  – global fallback level (ignored if `filter` present)
//! * `ansi`  – enable/disable colored output (default: true)
//! * `format.with_timestamp` – show timestamps (true) or hide them (false);
//!   omit for default (true)
//!
//! # Defaults (file absent or field omitted)
//! * level: `info`
//! * ansi: `true`
//! * timestamp: `true`
//!
//! # Minimal Example
//! ```yaml
//! level: info
//! ```
//!
//! # Filter Example
//! ```yaml
//! filter: info,mycrate::sub=debug
//! ```
//!
//! # Hide Timestamps
//! ```yaml
//! filter: info
//! format:
//!   with_timestamp: false
//! ```
//!
//! Unknown keys are ignored. Parse errors fall back to defaults. Diagnostics (successful initialization or existing subscriber) are emitted at `debug` level via the active tracing subscriber.
//!
//! # Diagnostics
//! * On success, emits `debug!(...)` with details of the logging configuration.
//! * If a subscriber is already installed, emits `debug!(...)` indicating this,
//!   and uses the existing configuration.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingSettings {
    pub filter: String,
    pub ansi: bool,
    pub with_timestamp: bool,
    pub source: String,
}

/// Configuration structure loaded from `logging.yml`.
#[derive(Deserialize, Default)]
struct LoggingConfig {
    level: Option<String>,
    ansi: Option<bool>,
    filter: Option<String>,
    format: Option<FormatConfig>,
}

#[derive(Deserialize, Default)]
struct FormatConfig {
    with_timestamp: Option<bool>,
}

fn select_config(preferred: &Path) -> (LoggingConfig, String) {
    let candidate = preferred.join("logging.yml");
    if candidate.exists() {
        match fs::read_to_string(&candidate) {
            Ok(txt) => (
                serde_yaml::from_str(&txt).unwrap_or_default(),
                candidate.display().to_string(),
            ),
            Err(err) => {
                debug!(path=%candidate.display(), error=%err, "Failed to read logging.yml; using defaults");
                (
                    LoggingConfig::default(),
                    format!("{} (read error, using defaults)", candidate.display()),
                )
            }
        }
    } else {
        let cwd = PathBuf::from("logging.yml");
        if cwd.exists() {
            match fs::read_to_string(&cwd) {
                Ok(txt) => (
                    serde_yaml::from_str(&txt).unwrap_or_default(),
                    cwd.display().to_string(),
                ),
                Err(err) => {
                    debug!(path=%cwd.display(), error=%err, "Failed to read cwd logging.yml; using defaults");
                    (
                        LoggingConfig::default(),
                        format!("{} (read error, using defaults)", cwd.display()),
                    )
                }
            }
        } else {
            (LoggingConfig::default(), "default".to_string())
        }
    }
}

#[cfg(not(feature = "serde"))]
fn select_config(_preferred: &Path) -> ((), String) {
    ((), "default(no-serde)".to_string())
}

/// Load logging settings (filter, ansi, timestamp) without installing a subscriber.
/// Public for testing.
pub fn load_logging_settings(preferred: &Path) -> LoggingSettings {
    #[cfg(feature = "serde")]
    {
        let (raw_cfg, source) = select_config(preferred);
        let filter = raw_cfg
            .filter
            .unwrap_or_else(|| raw_cfg.level.unwrap_or_else(|| "info".to_string()));
        let ansi = raw_cfg.ansi.unwrap_or(true);
        let with_timestamp = raw_cfg
            .format
            .as_ref()
            .and_then(|f| f.with_timestamp)
            .unwrap_or(true);
        return LoggingSettings {
            filter,
            ansi,
            with_timestamp,
            source,
        };
    }
    #[cfg(not(feature = "serde"))]
    {
        let (_raw, source) = select_config(preferred);
        return LoggingSettings {
            filter: "info".into(),
            ansi: true,
            with_timestamp: true,
            source,
        };
    }
}

/// Initialize tracing subscriber from a preferred directory or current working directory.
///
/// Search order:
/// 1. `<preferred>/logging.yml`
/// 2. `./logging.yml`
/// 3. Defaults (info level, ANSI enabled, with timestamps)
///
/// # Arguments
/// * `preferred` - Directory to search first (typically the config file's parent directory)
///
/// # Behavior
/// * Uses `try_init()` - silently ignores if a subscriber is already installed
/// * Emits a debug-level diagnostic line (via the tracing subscriber) on success or when already initialized (may be filtered)
/// * Never panics
pub fn init_from_dir(preferred: &Path) {
    let settings = load_logging_settings(preferred);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(settings.filter.clone()))
        .with_ansi(settings.ansi);
    let result = if settings.with_timestamp {
        subscriber.try_init()
    } else {
        subscriber.without_time().try_init()
    };
    if result.is_ok() {
        debug!(target="allora::logging", source=%settings.source, filter=%settings.filter, timestamp=%settings.with_timestamp, ansi=%settings.ansi, "Logging initialized");
    } else {
        debug!(target="allora::logging", wanted_filter=%settings.filter, "Logging subscriber already set; using existing configuration");
    }
}
