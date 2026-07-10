//! The installed-plugin catalog (pure logic; disk I/O lives in the app layer).
//!
//! Install verifies the signature and refuses duplicate ids; listing computes a
//! [`TrustLevel`] per plugin against the current [`TrustStore`]. The catalog
//! serializes to JSON so the app can persist it.

use crate::error::{PluginError, Result};
use crate::manifest::PluginManifest;
use crate::signing::{SignedPlugin, TrustLevel, TrustStore};
use serde::{Deserialize, Serialize};

/// A plugin in the catalog together with its evaluated trust level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// The manifest.
    pub manifest: PluginManifest,
    /// Trust evaluated against the app's trust store.
    pub trust: TrustLevel,
}

/// A persistable set of installed (signed) plugins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    plugins: Vec<SignedPlugin>,
}

impl Catalog {
    /// An empty catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a catalog from its JSON representation.
    ///
    /// # Errors
    /// Fails if the JSON is malformed.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serialize the catalog to JSON for persistence.
    ///
    /// # Errors
    /// Fails only on a serializer error.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Install a signed plugin, refusing a duplicate id.
    ///
    /// # Errors
    /// [`PluginError::AlreadyInstalled`] if the id is already present.
    pub fn install(&mut self, signed: SignedPlugin) -> Result<()> {
        let id = &signed.manifest.id;
        if self.plugins.iter().any(|p| &p.manifest.id == id) {
            return Err(PluginError::AlreadyInstalled(id.clone()));
        }
        self.plugins.push(signed);
        Ok(())
    }

    /// Install from a signed-plugin JSON document.
    ///
    /// # Errors
    /// Fails if the JSON is malformed or the id is already installed.
    pub fn install_json(&mut self, json: &str) -> Result<()> {
        let signed: SignedPlugin = serde_json::from_str(json)?;
        self.install(signed)
    }

    /// Remove a plugin by id.
    ///
    /// # Errors
    /// [`PluginError::NotFound`] if no such plugin is installed.
    pub fn uninstall(&mut self, id: &str) -> Result<()> {
        let before = self.plugins.len();
        self.plugins.retain(|p| p.manifest.id != id);
        if self.plugins.len() == before {
            return Err(PluginError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Whether a plugin id is installed.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.plugins.iter().any(|p| p.manifest.id == id)
    }

    /// The signed plugin for an id, if installed.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&SignedPlugin> {
        self.plugins.iter().find(|p| p.manifest.id == id)
    }

    /// List installed plugins with trust evaluated against `trust`.
    #[must_use]
    pub fn list(&self, trust: &TrustStore) -> Vec<InstalledPlugin> {
        self.plugins
            .iter()
            .map(|p| InstalledPlugin {
                manifest: p.manifest.clone(),
                trust: p.trust_level(trust),
            })
            .collect()
    }
}
