//! Vault composition — the *instance* side of the plugin split.
//!
//! A [`PluginManifest`](crate::PluginManifest) is a plugin *definition*; a
//! [`VaultComposition`] is how one vault *uses* plugins: which are enabled, each
//! one's per-vault settings and grid layout, plus the vault's visual
//! customization (theme, accent, icon). The storage layer persists this per
//! vault; the shell renders the dashboard from it instead of a hardcoded list.

use crate::manifest::TileLayout;
use serde::{Deserialize, Serialize};

/// Per-vault visual customization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultCustomization {
    /// Theme id override (e.g. `"forest"` / `"cyan"`); `None` = app default.
    #[serde(default)]
    pub theme: Option<String>,
    /// Accent color override as a CSS RGB triple string, e.g. `"217 119 6"`.
    #[serde(default)]
    pub accent: Option<String>,
    /// Icon/emoji shown on the vault's card and header.
    #[serde(default)]
    pub icon: Option<String>,
}

/// One enabled plugin instance within a vault.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnabledPlugin {
    /// The plugin's manifest id.
    pub plugin_id: String,
    /// Per-vault plugin settings (shape defined by the manifest's config schema).
    #[serde(default)]
    pub settings: serde_json::Value,
    /// The plugin's footprint on this vault's Bento grid.
    #[serde(default)]
    pub layout: TileLayout,
    /// Sort order on the dashboard.
    #[serde(default)]
    pub order: u32,
}

impl EnabledPlugin {
    /// Enable a plugin with default settings and the given layout/order.
    #[must_use]
    pub fn new(plugin_id: impl Into<String>, layout: TileLayout, order: u32) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            settings: serde_json::Value::Null,
            layout,
            order,
        }
    }
}

/// The full composition of one vault: customization + enabled plugins.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VaultComposition {
    /// Visual customization.
    #[serde(default)]
    pub customization: VaultCustomization,
    /// Enabled plugins, in dashboard order.
    #[serde(default)]
    pub plugins: Vec<EnabledPlugin>,
}

impl VaultComposition {
    /// Whether a plugin is enabled in this vault.
    #[must_use]
    pub fn has(&self, plugin_id: &str) -> bool {
        self.plugins.iter().any(|p| p.plugin_id == plugin_id)
    }

    /// Enable a plugin if not already present (appended at the end).
    pub fn enable(&mut self, plugin_id: impl Into<String>, layout: TileLayout) {
        let plugin_id = plugin_id.into();
        if !self.has(&plugin_id) {
            let order = self.plugins.len() as u32;
            self.plugins
                .push(EnabledPlugin::new(plugin_id, layout, order));
        }
    }

    /// Disable (remove) a plugin from the vault.
    pub fn disable(&mut self, plugin_id: &str) {
        self.plugins.retain(|p| p.plugin_id != plugin_id);
    }
}
