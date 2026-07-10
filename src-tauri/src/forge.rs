//! The BioSpark Forge runtime: the set of built-in (native) plugins, a bundled
//! store of declarative plugins signed by the BioSpark publisher key, and the
//! trust store used to badge them.
//!
//! The signing key here is a **development** key derived from a fixed seed so the
//! bundled store plugins verify as "Verified" out of the box and the whole
//! sign → trust → badge path is exercised end-to-end. A production build would
//! ship only the public key and sign the store offline.

use ed25519_dalek::SigningKey;
use quill_plugin::{
    Capability, CollectionSpec, FieldKind, FieldSpec, PluginCategory, PluginKind, PluginManifest,
    PluginScope, Pricing, SignedPlugin, TileLayout, TrustStore, UiPanel, UiSchema,
};

/// Dev-only BioSpark publisher seed (NOT a production secret).
const BIOSPARK_DEV_SEED: [u8; 32] = *b"biospark-forge-dev-signing-key!!";

/// The Forge: built-in native plugins + a signed declarative store + trust.
pub struct Forge {
    builtins: Vec<PluginManifest>,
    store: Vec<SignedPlugin>,
    trust: TrustStore,
}

impl Forge {
    /// Construct the Forge with built-ins, the signed store, and BioSpark trust.
    #[must_use]
    pub fn new() -> Self {
        let key = SigningKey::from_bytes(&BIOSPARK_DEV_SEED);
        let mut trust = TrustStore::new();
        trust.trust(key.verifying_key());

        let store = store_manifests()
            .into_iter()
            .filter_map(|m| SignedPlugin::sign(m, &key).ok())
            .collect();

        Self { builtins: builtin_manifests(), store, trust }
    }

    /// Built-in native plugin manifests.
    #[must_use]
    pub fn builtins(&self) -> &[PluginManifest] {
        &self.builtins
    }

    /// The signed declarative store plugins.
    #[must_use]
    pub fn store(&self) -> &[SignedPlugin] {
        &self.store
    }

    /// The trust store.
    #[must_use]
    pub fn trust(&self) -> &TrustStore {
        &self.trust
    }

    /// Resolve a manifest by id across built-ins and the store.
    #[must_use]
    pub fn manifest(&self, id: &str) -> Option<PluginManifest> {
        self.builtins
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .or_else(|| self.store.iter().find(|p| p.manifest.id == id).map(|p| p.manifest.clone()))
    }

    /// The signed store plugin for an id, if present.
    #[must_use]
    pub fn store_plugin(&self, id: &str) -> Option<&SignedPlugin> {
        self.store.iter().find(|p| p.manifest.id == id)
    }
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_manifests() -> Vec<PluginManifest> {
    vec![
        PluginManifest {
            id: "biospark.essay-version-control".into(),
            name: "Essay Version Control".into(),
            description: "Git-style drafts, revision history, and diffs.".into(),
            version: "1.0.0".into(),
            author: "BioSpark Studios".into(),
            icon: "📝".into(),
            category: PluginCategory::Advising,
            scope: PluginScope::Chamber,
            capabilities: vec![Capability::ReadEssays, Capability::WriteEssays],
            kind: PluginKind::Native { component: "EssayVersionControl".into() },
            default_layout: TileLayout { w: 1, h: 1 },
            config_schema: vec![],
            pricing: Pricing::Free,
        },
        PluginManifest {
            id: "biospark.timeline-weaver".into(),
            name: "Application Timeline Weaver".into(),
            description: "Deadlines and milestones on a chronological timeline.".into(),
            version: "1.0.0".into(),
            author: "BioSpark Studios".into(),
            icon: "🗓️".into(),
            category: PluginCategory::Advising,
            scope: PluginScope::Chamber,
            capabilities: vec![Capability::ReadMilestones, Capability::WriteMilestones],
            kind: PluginKind::Native { component: "TimelineWeaver".into() },
            default_layout: TileLayout { w: 1, h: 1 },
            config_schema: vec![],
            pricing: Pricing::Free,
        },
    ]
}

/// Declarative store plugins — defined entirely as data (no app code).
fn store_manifests() -> Vec<PluginManifest> {
    vec![
        collection_plugin(
            "biospark.recommendation-manager",
            "Recommendation Manager",
            "Track recommendation letters and their status.",
            "✉️",
            Pricing::Free,
            CollectionSpec {
                collection: "letters".into(),
                add_label: "Add letter".into(),
                title_field: "recommender".into(),
                subtitle_field: Some("status".into()),
                fields: vec![
                    field("recommender", "Recommender", FieldKind::Text, true),
                    field(
                        "status",
                        "Status",
                        FieldKind::Select {
                            options: vec![
                                "Requested".into(),
                                "Received".into(),
                                "Submitted".into(),
                            ],
                        },
                        false,
                    ),
                    field("due", "Due date", FieldKind::Date, false),
                ],
            },
        ),
        collection_plugin(
            "biospark.session-notes",
            "Session Notes",
            "Log advising-session notes per student.",
            "🗒️",
            Pricing::Free,
            CollectionSpec {
                collection: "notes".into(),
                add_label: "Add note".into(),
                title_field: "note".into(),
                subtitle_field: Some("date".into()),
                fields: vec![
                    field("date", "Date", FieldKind::Date, false),
                    field("note", "Note", FieldKind::LongText, true),
                ],
            },
        ),
        collection_plugin(
            "biospark.scholarship-tracker",
            "Scholarship Tracker",
            "Track scholarship applications, amounts, and deadlines.",
            "🎓",
            Pricing::Paid { tier: "pro".into(), price_cents: 500 },
            CollectionSpec {
                collection: "scholarships".into(),
                add_label: "Add scholarship".into(),
                title_field: "name".into(),
                subtitle_field: Some("status".into()),
                fields: vec![
                    field("name", "Name", FieldKind::Text, true),
                    field("amount", "Amount", FieldKind::Text, false),
                    field("deadline", "Deadline", FieldKind::Date, false),
                    field(
                        "status",
                        "Status",
                        FieldKind::Select {
                            options: vec![
                                "Researching".into(),
                                "Applied".into(),
                                "Awarded".into(),
                            ],
                        },
                        false,
                    ),
                ],
            },
        ),
    ]
}

fn field(key: &str, label: &str, kind: FieldKind, required: bool) -> FieldSpec {
    FieldSpec { key: key.into(), label: label.into(), kind, required }
}

fn collection_plugin(
    id: &str,
    name: &str,
    description: &str,
    icon: &str,
    pricing: Pricing,
    spec: CollectionSpec,
) -> PluginManifest {
    let title = spec.collection.clone();
    PluginManifest {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        version: "1.0.0".into(),
        author: "BioSpark Studios".into(),
        icon: icon.into(),
        category: PluginCategory::Advising,
        scope: PluginScope::Chamber,
        capabilities: vec![Capability::StorePluginRecords],
        kind: PluginKind::Declarative {
            ui: UiSchema { panels: vec![UiPanel { title, kind: quill_plugin::PanelKind::Collection(spec) }] },
        },
        default_layout: TileLayout { w: 1, h: 1 },
        config_schema: vec![],
        pricing,
    }
}
