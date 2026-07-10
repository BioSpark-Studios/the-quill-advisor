//! The declarative UI schema rendered for Forge (data-defined) plugins.
//!
//! The frontend ships a single generic renderer that turns a [`UiSchema`] into a
//! working panel. The common advising-tool shape — "a list of records you can
//! add to, backed by the vault" — is expressed with a [`PanelKind::Collection`],
//! whose records are stored through the capability-gated plugin record store. A
//! plugin can therefore be shipped as pure JSON.

use serde::{Deserialize, Serialize};

/// The root of a declarative plugin's UI: an ordered list of panels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSchema {
    /// Panels rendered top-to-bottom.
    pub panels: Vec<UiPanel>,
}

/// A single titled panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPanel {
    /// Panel heading.
    pub title: String,
    /// What the panel renders.
    pub kind: PanelKind,
}

/// The kinds of panel the generic renderer understands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PanelKind {
    /// A collection of records with an add-form and a list, backed by the vault.
    Collection(CollectionSpec),
    /// Static markdown/help text.
    Note {
        /// The markdown body.
        content: String,
    },
}

/// A record collection: the fields to capture, which field titles each item, and
/// the storage collection name the records live under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSpec {
    /// Storage collection key (namespaced per plugin in the vault database).
    pub collection: String,
    /// Label for the add button, e.g. "Add letter".
    pub add_label: String,
    /// The field whose value is shown as each record's title.
    pub title_field: String,
    /// Optional field whose value renders as a subtitle/metadata line.
    #[serde(default)]
    pub subtitle_field: Option<String>,
    /// The record's fields (used for both the add-form and display).
    pub fields: Vec<FieldSpec>,
}

/// One field in a form / config schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// Machine key stored in the record's JSON.
    pub key: String,
    /// Human label.
    pub label: String,
    /// Input kind.
    pub kind: FieldKind,
    /// Whether the field is required to submit.
    #[serde(default)]
    pub required: bool,
}

/// The input type of a [`FieldSpec`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    /// Single-line text.
    Text,
    /// Multi-line text.
    LongText,
    /// A calendar date (ISO `YYYY-MM-DD`).
    Date,
    /// A boolean toggle.
    Bool,
    /// A choice from a fixed list.
    Select {
        /// Allowed options.
        options: Vec<String>,
    },
}
