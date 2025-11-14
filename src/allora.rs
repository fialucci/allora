//! High-level application facade.
//!
//! Minimal builder API for constructing an `AlloraRuntime` from a configuration file.
//! Intended for embedding with only a couple of lines of code.
//!
//! # Quick Start
//! ```no_run
//! use allora::Allora;
//! let rt = Allora::new().run()?; // attempts to load ./allora.yml
//! # Ok::<_, allora::Error>(())
//! ```
//!
//! # Overview
//! The builder exposes three operations:
//! * `new()` – create a fresh builder (auto-discovery enabled)
//! * `with_config_file(path)` – provide an explicit configuration file path
//! * `run()` – build and return an `AlloraRuntime`
//!
//! If no path is supplied, `run()` will try a sensible default (`allora.yml`).
//! Errors surface via the returned `Result`.
//!
//! ## Custom Path
//! ```no_run
//! # use allora::Allora;
//! let rt = Allora::new().with_config_file("examples/basic/helloworld/allora.yml").run()?;
//! # Ok::<_, allora::Error>(())
//! ```
use crate::{
    channel::Channel,
    dsl::build,
    dsl::runtime::AlloraRuntime,
    error::{Error, Result},
};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Maximum depth to search parent directories when auto-discovering configuration.
/// Prevents infinite loops and excessive filesystem traversal in pathological cases
/// (e.g., symlink cycles or deeply nested directory structures).
const MAX_PARENT_SEARCH_DEPTH: u8 = 10;

/// `Allora` builder holds configuration inputs prior to runtime construction.
#[derive(Debug, Clone)]
pub struct Allora {
    config_path: Option<PathBuf>,
}

impl Default for Allora {
    fn default() -> Self {
        Self { config_path: None }
    }
}

impl Allora {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an explicit configuration file path (overrides auto-discovery).
    ///
    /// Accepts any type implementing `AsRef<Path>` (e.g. `&str`, `PathBuf`).
    /// Relative paths are resolved according to the current working directory.
    ///
    /// Does not validate path existence immediately; validation happens inside `run()`.
    pub fn with_config_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Build the runtime from the explicit or default configuration file.
    ///
    /// # Configuration Discovery
    ///
    /// When no explicit path is provided via `with_config_file()`:
    /// 1. First checks for `allora.yml` in the current working directory
    /// 2. If not found, ascends parent directories from the executable location
    ///    up to `MAX_PARENT_SEARCH_DEPTH` (10) levels
    /// 3. Returns an error if no config is found
    ///
    /// # Note on Testing
    ///
    /// The parent directory ascent from the executable location is difficult to
    /// test in isolation as it depends on the test executable's location.
    /// Tests focus on explicit path configuration which covers the majority
    /// of production use cases.
    pub fn run(self) -> Result<AlloraRuntime> {
        let explicit_opt = self.config_path.clone();
        let path = match &explicit_opt {
            Some(p) => p.clone(),
            None => resolve_default_config(),
        };

        if let Some(parent) = path.parent() {
            crate::logging::init_from_dir(parent);
        } else {
            crate::logging::init_from_dir(Path::new("."));
        }

        let exists = path.exists();
        let canonical_opt = if exists {
            path.canonicalize().ok()
        } else {
            None
        };

        // Log discovery/resolution with clearer semantics: canonical only if file exists.
        if explicit_opt.is_none() {
            info!(
                config.path=%path.display(),
                config.canonical=?canonical_opt.as_ref().map(|p| p.display().to_string()),
                canonical=canonical_opt.is_some(),
                auto=true,
                "Configuration auto-discovered"
            );
        } else {
            info!(
                config.path=%path.display(),
                config.canonical=?canonical_opt.as_ref().map(|p| p.display().to_string()),
                canonical=canonical_opt.is_some(),
                auto=false,
                "Configuration resolved"
            );
        }

        if !exists {
            return Err(Error::other(format!(
                "config file '{}' not found",
                path.display()
            )));
        }

        let rt = build(&path)?;
        for ch in rt.channels() {
            // TODO: channel kind is hard-coded ("in_memory"). When additional channel implementations are added,
            // expose a kind accessor on the Channel trait or extend Channel to inherit ChannelInfo, then log dynamically.
            debug!(
                channel.id = ch.id(),
                kind = "in_memory",
                "channel registered"
            );
        }
        debug!(
            channels = rt.channel_count(),
            filters = rt.filter_count(),
            "Runtime constructed"
        );
        Ok(rt)
    }
}

fn resolve_default_config() -> PathBuf {
    // Prefer CWD/allora.yml first.
    let cwd_candidate = PathBuf::from("allora.yml");
    if cwd_candidate.exists() {
        return cwd_candidate;
    }
    // Ascend from executable location (helps when running with --manifest-path from repo root).
    if let Ok(exe) = std::env::current_exe() {
        let mut dir_opt = exe.parent();
        let mut depth = 0u8;
        while let Some(dir) = dir_opt {
            if depth >= MAX_PARENT_SEARCH_DEPTH {
                break;
            }
            let candidate = dir.join("allora.yml");
            if candidate.exists() {
                return candidate;
            }
            dir_opt = dir.parent();
            depth += 1;
        }
    }
    // Fallback (will error later if truly absent).
    PathBuf::from("allora.yml")
}
