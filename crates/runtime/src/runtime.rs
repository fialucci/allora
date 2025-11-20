//! High-level application facade.
//!
//! Minimal builder API for constructing an `AlloraRuntime` from a configuration file.
//! Intended for embedding with only a couple of lines of code.
//!
//! # Quick Start
//! ```no_run
//! use allora_runtime::Runtime;
//! let rt = Runtime::new().run()?; // attempts to load ./allora.yml
//! # Ok::<_, allora_runtime::Error>(())
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
//! # use allora_runtime::Runtime;
//! let rt = Runtime::new().with_config_file("examples/basic/helloworld/allora.yml").run()?;
//! # Ok::<_, allora_runtime::Error>(())
//! ```
//!
//! # Service Wiring & The `#[service]` Macro
//! Services are described in configuration (YAML) under `service-activator:` blocks using a `ref-name` field.
//! Example YAML fragment:
//! ```yaml
//! version: 1
//! service-activators:
//!   - ref-name: hello_world
//!     from: inbound.orders
//!     to: vetted.orders
//! ```
//! At runtime, wiring matches each `ref-name` value against inventory descriptors submitted via
//! the `#[service]` attribute macro. The macro registers a constructor closure that builds a
//! single shared instance (singleton) of your type via its zero-arg `new()` and invokes its async `process` method.
//!
//! ## Using `#[service]`
//! Apply the attribute to an inherent `impl` block that contains a zero-arg `new()` method.
//! If `name` is omitted the concrete type name (with spaces removed) is used.
//!
//! Supported forms:
//! * `#[service]` – implicit name = type name
//! * `#[service(name = "custom")]` – explicit reference name matching YAML `ref-name`
//!
//! Example:
//! ```ignore
//! use allora::{service, Service, Exchange, error::Result};
//! #[derive(Debug)]
//! struct Uppercase;
//! impl Uppercase { pub fn new() -> Self { Self } }
//! #[service(name="uppercase")]
//! impl Uppercase {}
//! #[async_trait::async_trait]
//! impl Service for Uppercase {
//!     async fn process(&self, exchange: &mut Exchange) -> Result<()> {
//!         if let Some(body) = exchange.in_msg.body_text() { exchange.out_msg = Some(Message::from_text(body.to_uppercase())); }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Descriptor Wiring Criteria
//! A service is wired only if:
//! 1. Its `ref-name` matches a registered descriptor `name`.
//! 2. Both `from` and `to` channel IDs exist in the runtime.
//! 3. The inbound channel is currently a direct channel (other kinds may be supported later).
//!
//! # Logging Fields
//! Structured fields emitted during build:
//! * `config.path` / `config.canonical` – discovery details
//! * `service_activator.ref_name`, `inbound`, `outbound` – wiring decisions
//! * `wired.count` – number of services wired
//! * `channel.id`, `kind` – registered channels
//! * `descriptor.impl` (legacy field name in traces) now maps to descriptor `name`.
//!
//! # Testing Notes
//! * Use #[tokio::test] for async service wiring tests.
//! * Macro UI tests (see `tests/macro/`) validate diagnostics & naming.
//! * All services share a single instance; design internal mutability carefully.
use crate::{
    all_service_descriptors,
    channel::Channel,
    dsl::build,
    dsl::runtime::AlloraRuntime,
    error::{Error, Result},
    service,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, trace};

/// `Allora` builder holds configuration inputs prior to runtime construction.
#[derive(Debug, Clone)]
pub struct Runtime {
    config_path: Option<PathBuf>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self { config_path: None }
    }
}

impl Runtime {
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
            return Err(Error::runtime(format!(
                "config file '{}' not found",
                path.display()
            )));
        }

        let rt = build(&path)?;
        wire_services(&rt)?;
        debug!(
            channels = rt.channel_count(),
            filters = rt.filter_count(),
            "Runtime constructed"
        );
        Ok(rt)
    }
}

pub fn wire_services(rt: &AlloraRuntime) -> Result<()> {
    let descriptors = all_service_descriptors();
    debug!(
        service_activator.processors = rt.service_processor_count(),
        descriptors = descriptors.len(),
        "service wiring start"
    );
    for d in &descriptors {
        trace!(descriptor.impl = d.name, "service descriptor loaded");
    }
    let mut service_activator_wirings: Vec<(
        Arc<dyn Channel>,
        Arc<dyn Channel>,
        Arc<dyn service::Service>,
        String,
    )> = Vec::new();
    for sp in rt.service_activator_processors().iter() {
        let name_key = sp.ref_name();
        trace!(
            service_activator.ref_name = name_key,
            service.id = sp.id(),
            from = sp.from(),
            to = sp.to(),
            "evaluating service processor"
        );
        for desc in descriptors.iter() {
            if desc.name == name_key {
                trace!(service_activator.ref_name = name_key, "descriptor matched");
                if rt.channel_by_id(sp.from()).is_some() && rt.channel_by_id(sp.to()).is_some() {
                    let inbound_arc_opt = rt.channels_slice().iter().find(|c| c.id() == sp.from());
                    let outbound_arc_opt = rt.channels_slice().iter().find(|c| c.id() == sp.to());
                    if let (Some(in_arc), Some(out_arc)) = (inbound_arc_opt, outbound_arc_opt) {
                        debug!(
                            service_activator.ref_name = name_key,
                            inbound = sp.from(),
                            outbound = sp.to(),
                            "channels resolved – scheduling wiring"
                        );
                        let proc_arc = (desc.constructor)();
                        service_activator_wirings.push((
                            in_arc.clone(),
                            out_arc.clone(),
                            proc_arc,
                            name_key.to_string(),
                        ));
                    } else {
                        debug!(
                            service_activator.ref_name = name_key,
                            inbound_found = inbound_arc_opt.is_some(),
                            outbound_found = outbound_arc_opt.is_some(),
                            "channel resolution failed – wiring skipped"
                        );
                    }
                } else {
                    debug!(
                        service_activator.ref_name = name_key,
                        "channel ids not found – skipped"
                    );
                }
            }
        }
    }
    if service_activator_wirings.is_empty() {
        info!("no services wired (none matched or channels missing)");
    } else {
        info!(
            wired.count = service_activator_wirings.len(),
            "service wiring collected"
        );
    }
    for (in_arc, out_arc, proc_arc, name_key) in service_activator_wirings.into_iter() {
        if let Some(inbound_direct) = in_arc.as_any().downcast_ref::<crate::DirectChannel>() {
            let outbound_arc_dyn = out_arc.clone();
            let inbound_id = inbound_direct.id().to_string();
            let name_key_closure = name_key.clone();
            let proc_shared = proc_arc.clone();
            let sub_count = inbound_direct.subscribe(move |exchange| {
                let outbound_clone = outbound_arc_dyn.clone();
                let proc_task = proc_shared.clone();
                let name_key_val = name_key_closure.clone();
                tokio::spawn(async move {
                    let mut ex_mut = exchange;
                    if let Err(err) = proc_task.process(&mut ex_mut).await {
                        tracing::error!(target="allora::service", service.impl=%name_key_val, error=%err, "Service async processing failed");
                        return;
                    }
                    if let Err(err) = outbound_clone.send(ex_mut).await {
                        tracing::error!(target="allora::service", service.impl=%name_key_val, error=%err, "Outbound channel send failed");
                    }
                });
                Ok(())
            });
            debug!(
                service_activator.ref_name = name_key,
                inbound = inbound_id,
                subscribers = sub_count,
                "service wired"
            );
        } else {
            debug!(
                service_activator.ref_name = name_key,
                inbound_id = in_arc.id(),
                "inbound channel not direct – skipping wiring"
            );
        }
    }
    for ch in rt.channels() {
        debug!(channel.id = ch.id(), kind = ch.kind(), "channel registered");
    }
    debug!(
        services.wired = rt.service_processor_count(),
        "runtime wiring complete"
    );
    Ok(())
}

fn resolve_default_config() -> PathBuf {
    use std::env;

    // 0. CLI override: --runtime <path> or --runtime=/path
    //
    // Example:
    //   cargo run --manifest-path examples/basic/http/Cargo.toml -- \
    //     --runtime examples/basic/http/allora.yml
    //
    // or
    //
    //   cargo run -- ... --runtime=examples/basic/http/allora.yml
    let mut args = env::args().skip(1); // skip program name
    let mut runtime_override: Option<String> = None;

    while let Some(arg) = args.next() {
        if arg == "--runtime" {
            if let Some(val) = args.next() {
                runtime_override = Some(val);
            }
            break;
        } else if let Some(rest) = arg.strip_prefix("--runtime=") {
            runtime_override = Some(rest.to_string());
            break;
        }
    }

    if let Some(raw) = runtime_override {
        let p = PathBuf::from(raw);
        // If it's a directory, assume allora.yml inside it.
        if p.is_dir() {
            return p.join("allora.yml");
        } else {
            return p;
        }
    }

    // 1. Optional env override (useful for CI or scripts)
    if let Ok(raw) = env::var("ALLORA_CONFIG") {
        let p = PathBuf::from(raw);
        if p.is_dir() {
            return p.join("allora.yml");
        } else {
            return p;
        }
    }

    // 2. Prefer ./allora.yml in the current working directory (dev-friendly)
    let cwd_candidate = PathBuf::from("allora.yml");
    if cwd_candidate.exists() {
        return cwd_candidate;
    }

    // 3. Then try <directory_of_executable>/allora.yml (release-friendly)
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("allora.yml");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // 4. Fallback (will cause a clear error in `run()` if missing)
    PathBuf::from("allora.yml")
}
