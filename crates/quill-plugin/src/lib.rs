//! # quill-plugin
//!
//! The **BioSpark Forge** plugin domain — pure, no I/O:
//!
//! * [`manifest`] — the plugin contract: id, capabilities, scope, kind
//!   (native component vs declarative UI schema), config schema, pricing.
//! * [`ui_schema`] — the declarative UI a data-defined Forge plugin renders.
//! * [`signing`] — ed25519 signing + a trust store for publisher verification.
//! * [`catalog`] — the installable/uninstallable plugin catalog.
//!
//! The whole point is the split between a plugin *definition* (a manifest, code
//! or data) and its *instance in a vault* (which the storage layer holds): that
//! keeps vault features composable and removable, instead of welded into the
//! shell.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod catalog;
pub mod error;
pub mod manifest;
pub mod signing;
pub mod ui_schema;
pub mod vault_config;

pub use catalog::{Catalog, InstalledPlugin};
pub use error::{PluginError, Result};
pub use manifest::{
    Capability, PluginCategory, PluginKind, PluginManifest, PluginScope, Pricing, TileLayout,
};
pub use signing::{SignedPlugin, TrustLevel, TrustStore};
pub use ui_schema::{CollectionSpec, FieldKind, FieldSpec, PanelKind, UiPanel, UiSchema};
pub use vault_config::{EnabledPlugin, VaultComposition, VaultCustomization};

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn sample_manifest() -> PluginManifest {
        PluginManifest {
            id: "biospark.recommendation-manager".into(),
            name: "Recommendation Manager".into(),
            description: "Track recommendation letters.".into(),
            version: "1.0.0".into(),
            author: "BioSpark Studios".into(),
            icon: "✉️".into(),
            category: PluginCategory::Advising,
            scope: PluginScope::Chamber,
            capabilities: vec![Capability::StorePluginRecords],
            kind: PluginKind::Declarative {
                ui: UiSchema {
                    panels: vec![UiPanel {
                        title: "Letters".into(),
                        kind: PanelKind::Collection(CollectionSpec {
                            collection: "letters".into(),
                            add_label: "Add letter".into(),
                            title_field: "recommender".into(),
                            subtitle_field: Some("status".into()),
                            fields: vec![
                                FieldSpec {
                                    key: "recommender".into(),
                                    label: "Recommender".into(),
                                    kind: FieldKind::Text,
                                    required: true,
                                },
                                FieldSpec {
                                    key: "status".into(),
                                    label: "Status".into(),
                                    kind: FieldKind::Select {
                                        options: vec!["Requested".into(), "Received".into()],
                                    },
                                    required: false,
                                },
                            ],
                        }),
                    }],
                },
            },
            default_layout: TileLayout { w: 1, h: 1 },
            config_schema: vec![],
            pricing: Pricing::Free,
        }
    }

    #[test]
    fn manifest_json_roundtrips_and_capabilities_work() {
        let m = sample_manifest();
        let json = serde_json::to_string(&m).unwrap();
        let back: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
        assert!(back.has_capability(Capability::StorePluginRecords));
        assert!(!back.has_capability(Capability::UseAi));
    }

    #[test]
    fn declarative_plugin_parses_from_pure_json() {
        // A Forge plugin shipped as data alone (no code) deserializes fully.
        let json = r#"{
            "id": "acme.notes",
            "name": "Session Notes",
            "description": "Jot notes.",
            "version": "0.1.0",
            "author": "Acme",
            "icon": "📝",
            "category": "advising",
            "scope": "chamber",
            "capabilities": ["store_plugin_records"],
            "kind": { "type": "declarative", "ui": { "panels": [
                { "title": "Notes", "kind": { "type": "collection",
                  "collection": "notes", "add_label": "Add note", "title_field": "text",
                  "fields": [ { "key": "text", "label": "Note", "kind": { "type": "long_text" }, "required": true } ] } }
            ] } }
        }"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "acme.notes");
        assert!(matches!(m.kind, PluginKind::Declarative { .. }));
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let signed = SignedPlugin::sign(sample_manifest(), &key).unwrap();

        let mut trust = TrustStore::new();
        trust.trust(key.verifying_key());
        assert_eq!(signed.trust_level(&trust), TrustLevel::Verified);
    }

    #[test]
    fn untrusted_publisher_is_not_verified() {
        let publisher = SigningKey::from_bytes(&[7u8; 32]);
        let signed = SignedPlugin::sign(sample_manifest(), &publisher).unwrap();
        // Trust store holds a *different* key.
        let mut trust = TrustStore::new();
        trust.trust(SigningKey::from_bytes(&[9u8; 32]).verifying_key());
        assert_eq!(signed.trust_level(&trust), TrustLevel::Untrusted);
    }

    #[test]
    fn tampering_breaks_the_signature() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut signed = SignedPlugin::sign(sample_manifest(), &key).unwrap();
        // Tamper with the manifest after signing.
        signed.manifest.name = "Malicious rename".into();
        let mut trust = TrustStore::new();
        trust.trust(key.verifying_key());
        assert_eq!(signed.trust_level(&trust), TrustLevel::Untrusted);
    }

    #[test]
    fn unsigned_plugin_reports_unsigned() {
        let signed = SignedPlugin::unsigned(sample_manifest());
        assert_eq!(signed.trust_level(&TrustStore::new()), TrustLevel::Unsigned);
    }

    #[test]
    fn catalog_install_dedup_uninstall_and_trust() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let signed = SignedPlugin::sign(sample_manifest(), &key).unwrap();
        let mut trust = TrustStore::new();
        trust.trust(key.verifying_key());

        let mut cat = Catalog::new();
        cat.install(signed.clone()).unwrap();
        assert!(cat.contains("biospark.recommendation-manager"));
        // Duplicate id refused.
        assert!(matches!(
            cat.install(signed),
            Err(PluginError::AlreadyInstalled(_))
        ));

        let listed = cat.list(&trust);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trust, TrustLevel::Verified);

        // Persist + reload preserves the catalog.
        let json = cat.to_json().unwrap();
        let reloaded = Catalog::from_json(&json).unwrap();
        assert!(reloaded.contains("biospark.recommendation-manager"));

        cat.uninstall("biospark.recommendation-manager").unwrap();
        assert!(!cat.contains("biospark.recommendation-manager"));
        assert!(matches!(
            cat.uninstall("nope"),
            Err(PluginError::NotFound(_))
        ));
    }
}
