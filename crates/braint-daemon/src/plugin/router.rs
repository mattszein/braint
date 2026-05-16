//! Verb router — maps verb names to the plugin that owns them.

use std::collections::HashMap;

/// Maps verb name → plugin name that owns it.
///
/// Enforces uniqueness: each verb may be registered by at most one plugin.
#[derive(Default)]
pub struct VerbRouter {
    /// verb name → plugin name
    map: HashMap<String, String>,
}

impl VerbRouter {
    /// Register a verb for a plugin.
    ///
    /// Returns `Err` if the verb is already claimed by another plugin.
    pub fn register(&mut self, plugin_name: &str, verb: &str) -> Result<(), String> {
        if let Some(owner) = self.map.get(verb) {
            return Err(format!(
                "verb '{}' already registered by plugin '{}'",
                verb, owner
            ));
        }
        self.map.insert(verb.to_string(), plugin_name.to_string());
        Ok(())
    }

    /// Return the plugin name that owns `verb`, if any.
    pub fn owner(&self, verb: &str) -> Option<&str> {
        self.map.get(verb).map(|s| s.as_str())
    }

    /// Return `true` if any plugin has registered `verb`.
    pub fn owns(&self, verb: &str) -> bool {
        self.map.contains_key(verb)
    }
}
