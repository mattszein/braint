//! Plugin subsystem — load, route, and lifecycle-manage external plugin binaries.

pub mod handle;
pub mod lifecycle;
pub mod router;

use braint_proto::{
    PluginVerbRequest, PluginVerbResponse,
    plugin::{PLUGIN_API_VERSION, PluginManifest},
};
use handle::PluginHandle;
use router::VerbRouter;
use std::collections::HashMap;
use std::path::PathBuf;

/// Owns all loaded plugins and routes verb invocations to them.
///
/// Stored as `Arc<PluginManager>` in [`DaemonState`]; all fields must be `Send + Sync`.
pub struct PluginManager {
    /// plugin name → live handle
    handles: HashMap<String, PluginHandle>,
    /// verb name → plugin name
    router: VerbRouter,
    /// verb name → `VerbManifest::takes_entry_id`
    verb_takes_entry_id: HashMap<String, bool>,
}

impl PluginManager {
    /// Construct an empty manager (no plugins loaded).
    pub fn empty() -> Self {
        Self {
            handles: HashMap::new(),
            router: VerbRouter::default(),
            verb_takes_entry_id: HashMap::new(),
        }
    }

    /// Scan `plugin_dirs` for executable files, fetch their manifests, and spawn them.
    ///
    /// Directories that don't exist are silently skipped.
    /// Individual plugin load failures are logged as warnings but do not abort the scan.
    pub async fn load(
        plugin_dirs: &[PathBuf],
    ) -> Result<Self, crate::error::DaemonError> {
        let mut manager = Self::empty();

        for dir in plugin_dirs {
            if !dir.exists() {
                continue;
            }
            let read_dir = match std::fs::read_dir(dir) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("cannot read plugin dir {}: {e}", dir.display());
                    continue;
                }
            };
            for entry_result in read_dir {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // On Unix, only load files with at least one execute bit set.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    match std::fs::metadata(&path) {
                        Ok(meta) => {
                            if meta.permissions().mode() & 0o111 == 0 {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                match manager.load_plugin(&path).await {
                    Ok(()) => tracing::info!("loaded plugin: {}", path.display()),
                    Err(e) => tracing::warn!("failed to load plugin {}: {e}", path.display()),
                }
            }
        }

        Ok(manager)
    }

    /// Load a single plugin binary: fetch manifest, validate, register verbs, spawn.
    async fn load_plugin(
        &mut self,
        binary: &PathBuf,
    ) -> Result<(), crate::error::DaemonError> {
        let manifest = lifecycle::fetch_manifest(binary).await?;
        self.validate_manifest(&manifest)?;
        let name = manifest.name.clone();
        for verb in &manifest.verbs {
            self.router
                .register(&name, &verb.name)
                .map_err(crate::error::DaemonError::PluginGovernance)?;
            self.verb_takes_entry_id
                .insert(verb.name.clone(), verb.takes_entry_id);
        }
        let handle = lifecycle::spawn_plugin(binary, manifest).await?;
        self.handles.insert(name, handle);
        Ok(())
    }

    /// Validate a manifest against governance rules.
    fn validate_manifest(
        &self,
        m: &PluginManifest,
    ) -> Result<(), crate::error::DaemonError> {
        if m.api_version != PLUGIN_API_VERSION {
            return Err(crate::error::DaemonError::PluginGovernance(format!(
                "plugin '{}' api_version {} != daemon's {}",
                m.name, m.api_version, PLUGIN_API_VERSION
            )));
        }
        // Event topics must start with "<plugin_name>."
        for topic in &m.events_subscribed {
            if !topic.starts_with(&format!("{}.", m.name)) {
                return Err(crate::error::DaemonError::PluginGovernance(format!(
                    "plugin '{}' event topic '{}' must start with '{}.'" ,
                    m.name, topic, m.name
                )));
            }
        }
        Ok(())
    }

    /// Return `true` if any loaded plugin owns `verb`.
    pub fn owns_verb(&self, verb: &str) -> bool {
        self.router.owns(verb)
    }

    /// Return `true` if the verb's manifest declares `takes_entry_id = true`.
    pub fn verb_takes_entry_id(&self, verb: &str) -> bool {
        self.verb_takes_entry_id.get(verb).copied().unwrap_or(false)
    }

    /// Route a verb invocation to the owning plugin and return its response.
    pub async fn route_verb(
        &self,
        verb: &str,
        req: PluginVerbRequest,
    ) -> Result<PluginVerbResponse, crate::error::DaemonError> {
        let plugin_name = self.router.owner(verb).ok_or_else(|| {
            crate::error::DaemonError::PluginGovernance(format!(
                "no plugin owns verb '{verb}'"
            ))
        })?;
        let handle = self
            .handles
            .get(plugin_name)
            .ok_or_else(|| crate::error::DaemonError::PluginDead(plugin_name.to_string()))?;
        handle.call_verb(req).await
    }

    /// Send SIGTERM to all plugins and wait up to 5 seconds for each to exit.
    pub async fn shutdown(&self) {
        for (name, handle) in &self.handles {
            let mut child = handle.child.lock().await;

            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    // SAFETY: kill(2) is safe to call with a valid pid and SIGTERM (15).
                    unsafe {
                        libc::kill(pid as libc::pid_t, libc::SIGTERM);
                    }
                }
            }

            let timeout = tokio::time::Duration::from_secs(5);
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(_) => tracing::info!("plugin {name} shut down cleanly"),
                Err(_) => {
                    tracing::warn!("plugin {name} did not exit in time, killing");
                    let _ = child.kill().await;
                }
            }
        }
    }
}
