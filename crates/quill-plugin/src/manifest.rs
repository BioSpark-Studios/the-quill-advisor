//! The plugin manifest — the contract every vault-facing feature is described by.
//!
//! A manifest is pure data (serde/JSON). Built-in plugins ship a manifest whose
//! [`PluginKind::Native`] names a React component in the app; Forge plugins ship
//! a manifest whose [`PluginKind::Declarative`] carries a [`UiSchema`] rendered
//! generically — so a plugin can be defined as *data*, not code. This is the
//! separation that keeps features from being welded into the shell.

use crate::ui_schema::UiSchema;
use serde::{Deserialize, Serialize};

/// A fully described plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Globally unique id, e.g. `"biospark.essay-version-control"`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// One-line description shown in the store and manager.
    pub description: String,
    /// Semantic version string.
    pub version: String,
    /// Author / publisher, e.g. `"BioSpark Studios"`.
    pub author: String,
    /// Emoji or asset reference used as the tile/store icon.
    pub icon: String,
    /// Grouping category.
    pub category: PluginCategory,
    /// Whether the plugin operates on the whole vault or a single chamber.
    pub scope: PluginScope,
    /// What vault data / gates the plugin is permitted to touch. Surfaced to the
    /// counselor in the Addon Manager and enforced at the command layer.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// How the plugin is implemented (native component vs declarative UI schema).
    pub kind: PluginKind,
    /// Default tile footprint on the Magic Bento grid.
    #[serde(default)]
    pub default_layout: TileLayout,
    /// Declarative configuration fields exposed in the Addon Manager.
    #[serde(default)]
    pub config_schema: Vec<crate::ui_schema::FieldSpec>,
    /// Monetization info (Forge creator economy).
    #[serde(default)]
    pub pricing: Pricing,
}

impl PluginManifest {
    /// Whether the plugin declares a given capability.
    #[must_use]
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Canonical bytes used for signing/verification (stable serde field order).
    ///
    /// # Errors
    /// Fails only if the manifest cannot be serialized to JSON.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// Plugin grouping, used for store/manager organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    /// Academic advising tools.
    Advising,
    /// Business / operations tools.
    Business,
    /// AI agents and assistants.
    Ai,
    /// Analytics & reporting.
    Analytics,
    /// Visual / dashboard widgets.
    Visual,
    /// Anything else.
    Other,
}

/// Where a plugin operates in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginScope {
    /// Operates on the whole vault (e.g. billing).
    Vault,
    /// Operates within a single chamber (e.g. essay drafting).
    Chamber,
}

/// A permission a plugin declares. These map onto the vault's data domains and
/// gate model — a plugin can only perform a data op it holds the capability for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Read student records.
    ReadStudentData,
    /// Create/modify student records.
    WriteStudentData,
    /// Read essays and revision history.
    ReadEssays,
    /// Commit essay revisions.
    WriteEssays,
    /// Read milestones.
    ReadMilestones,
    /// Create/modify milestones.
    WriteMilestones,
    /// Invoke AI agents / the Omni-Route router.
    UseAi,
    /// Read the billing ledger.
    ReadBilling,
    /// Write billing entries.
    WriteBilling,
    /// Read/write the plugin's own namespaced record store (generic plugins).
    StorePluginRecords,
}

impl Capability {
    /// A short human phrase for the Addon Manager permission list.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Capability::ReadStudentData => "Read student data",
            Capability::WriteStudentData => "Modify student data",
            Capability::ReadEssays => "Read essays",
            Capability::WriteEssays => "Write essays",
            Capability::ReadMilestones => "Read milestones",
            Capability::WriteMilestones => "Manage milestones",
            Capability::UseAi => "Use AI",
            Capability::ReadBilling => "Read billing",
            Capability::WriteBilling => "Write billing",
            Capability::StorePluginRecords => "Store its own records",
        }
    }
}

/// How a plugin is implemented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginKind {
    /// A built-in plugin backed by a named React component in the app.
    Native {
        /// The component key registered in the frontend plugin registry.
        component: String,
    },
    /// A Forge plugin defined entirely by a declarative UI schema (no code).
    Declarative {
        /// The UI to render generically.
        ui: UiSchema,
    },
}

/// A plugin's footprint on the Magic Bento grid (in grid cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileLayout {
    /// Column span.
    pub w: u8,
    /// Row span.
    pub h: u8,
}

impl Default for TileLayout {
    fn default() -> Self {
        Self { w: 1, h: 1 }
    }
}

/// Monetization metadata for the Forge creator economy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Pricing {
    /// Free to install.
    #[default]
    Free,
    /// Paid, in whole cents, under a named tier.
    Paid {
        /// Tier name (e.g. "pro").
        tier: String,
        /// Price in cents (USD).
        price_cents: u32,
    },
}
