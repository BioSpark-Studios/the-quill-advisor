//! Tauri IPC commands. The command names and argument shapes mirror
//! `ui/src/lib/ipc.ts`. Each maps UI actions onto the real `quill-*` crates.

use crate::state::AppState;
use quill_ai::Message;
use quill_core::authz::ChamberAiAuthorization;
use quill_core::vault::VaultNode;
use quill_core::VaultId;
use quill_plugin::{
    Capability, Catalog, PluginManifest, TrustLevel, VaultComposition, VaultCustomization,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// A vault card as consumed by the Master Vault board.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultCardDto {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub students: i64,
    pub chambers: i64,
    /// Retainer hours remaining (retainer minus time logged), rounded to the
    /// nearest tenth of an hour.
    pub hour_balance: f64,
    pub last_activity: String,
    /// Vault crest/icon, sourced from the vault composition's customization.
    pub icon: Option<String>,
    /// Accent color (CSS RGB triple), sourced from customization.
    pub accent: Option<String>,
}

/// Card metadata persisted alongside a vault node (settings key `card:{id}`).
/// The hour balance shown on the card is computed live from the vault's
/// billing ledger, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CardMeta {
    stage: String,
    students: i64,
    chambers: i64,
}

/// The Quill chat reply returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatReplyDto {
    pub text: String,
    pub provider: String,
    pub model: String,
}

/// One essay revision returned to the Essay Version Control panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EssayVersionDto {
    pub id: String,
    pub essay_id: String,
    pub seq: i64,
    pub message: String,
    pub body: String,
}

impl From<quill_storage::EssayVersion> for EssayVersionDto {
    fn from(v: quill_storage::EssayVersion) -> Self {
        Self { id: v.id, essay_id: v.essay_id, seq: v.seq, message: v.message, body: v.body }
    }
}

/// A milestone on a student's application timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneDto {
    pub id: String,
    pub title: String,
    pub due_at: Option<String>,
    pub done: bool,
}

impl From<quill_storage::Milestone> for MilestoneDto {
    fn from(m: quill_storage::Milestone) -> Self {
        Self { id: m.id, title: m.title, due_at: m.due_at, done: m.done }
    }
}

/// Derive a stable chamber `VaultId` from the UI's chamber string so per-chamber
/// AI authorization can be looked up in `core_db`.
fn chamber_vault_id(chamber_id: &str) -> VaultId {
    VaultId(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, chamber_id.as_bytes()))
}

async fn card_meta(state: &AppState, id: VaultId) -> CardMeta {
    match state.core.get_setting(&format!("card:{id}")).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => CardMeta::default(),
    }
}

/// Retainer hours remaining for a vault, rounded to the nearest tenth. A vault
/// that can't be opened (or has no retainer set) simply shows zero rather than
/// failing the whole card list.
async fn hour_balance(state: &AppState, id: VaultId) -> f64 {
    let Ok(db) = state.vaults.vault(id).await else {
        return 0.0;
    };
    let retainer = db.retainer_minutes().await.unwrap_or(0);
    let used = db.billing_minutes_used().await.unwrap_or(0);
    ((retainer - used) as f64 / 60.0 * 10.0).round() / 10.0
}

/// Backend health check, run during the splash.
#[tauri::command]
pub async fn health_check(state: State<'_, AppState>) -> Result<bool, String> {
    state.core.master().await.map(|m| m.is_some()).map_err(|e| e.to_string())
}

/// List all Counselor-level vaults as cards for the Master Vault board.
#[tauri::command]
pub async fn get_vault_hierarchy(state: State<'_, AppState>) -> Result<Vec<VaultCardDto>, String> {
    let children = state.core.children(state.master_id).await.map_err(|e| e.to_string())?;
    let mut cards = Vec::new();
    for rec in children {
        let meta = card_meta(&state, rec.node.id).await;
        // Customization is sourced from each vault's composition (the SSoT).
        let custom = state.core.vault_composition(rec.node.id).await.unwrap_or_default().customization;
        cards.push(VaultCardDto {
            id: rec.node.id.to_string(),
            name: rec.node.name,
            stage: if meta.stage.is_empty() { "active".into() } else { meta.stage },
            students: meta.students,
            chambers: meta.chambers,
            hour_balance: hour_balance(&state, rec.node.id).await,
            last_activity: rec.created_at.format("%Y-%m-%d").to_string(),
            icon: custom.icon,
            accent: custom.accent,
        });
    }
    Ok(cards)
}

/// Create a new Counselor vault under the Master, storing its card metadata.
#[tauri::command]
pub async fn create_vault(
    state: State<'_, AppState>,
    name: String,
    stage: String,
    customization: Option<VaultCustomization>,
    template: Option<Vec<String>>,
) -> Result<VaultCardDto, String> {
    let master = state
        .core
        .get_node(state.master_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("master vault missing")?;
    let node = VaultNode::child_of(&master.node, VaultId::new(), &name).map_err(|e| e.to_string())?;
    state
        .core
        .insert_node(&node, Some(&format!("vaults/{}.db", node.id)), &quill_core::GateConfig::sealed())
        .await
        .map_err(|e| e.to_string())?;
    let meta = CardMeta { stage: stage.clone(), ..Default::default() };
    state
        .core
        .upsert_setting(
            &format!("card:{}", node.id),
            &serde_json::to_string(&meta).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;

    // Seed the vault composition from the chosen customization + starter template.
    let mut composition = VaultComposition {
        customization: customization.unwrap_or_default(),
        ..Default::default()
    };
    if let Some(ids) = template {
        for id in ids {
            if let Some(manifest) = resolve_manifest(&state, &id).await {
                composition.enable(manifest.id, manifest.default_layout);
            }
        }
    }
    let custom = composition.customization.clone();
    state
        .core
        .set_vault_composition(node.id, &composition)
        .await
        .map_err(|e| e.to_string())?;

    Ok(VaultCardDto {
        id: node.id.to_string(),
        name,
        stage,
        students: 0,
        chambers: 0,
        hour_balance: 0.0,
        last_activity: "just now".into(),
        icon: custom.icon,
        accent: custom.accent,
    })
}

/// Move a vault card to a different engagement stage.
#[tauri::command]
pub async fn move_vault(
    state: State<'_, AppState>,
    id: String,
    stage: String,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let mut meta = card_meta(&state, vid).await;
    meta.stage = stage;
    state
        .core
        .upsert_setting(
            &format!("card:{id}"),
            &serde_json::to_string(&meta).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())
}

/// Whether the counselor has enabled Quantum Quill in this chamber.
#[tauri::command]
pub async fn chamber_ai_enabled(
    state: State<'_, AppState>,
    chamber_id: String,
) -> Result<bool, String> {
    let vid = chamber_vault_id(&chamber_id);
    state.core.ai_authorization(vid).await.map(|a| a.enabled).map_err(|e| e.to_string())
}

/// Toggle Quantum Quill authorization for a chamber. Enabling both flips the
/// master switch and allow-lists the Quill agent.
#[tauri::command]
pub async fn set_chamber_ai(
    state: State<'_, AppState>,
    chamber_id: String,
    enabled: bool,
) -> Result<(), String> {
    let vid = chamber_vault_id(&chamber_id);
    let mut auth = ChamberAiAuthorization::disabled(vid);
    auth.set_enabled(enabled);
    if enabled {
        auth.authorize(state.agent.id());
    }
    state.core.set_ai_authorization(&auth).await.map_err(|e| e.to_string())
}

/// Commit a new essay revision into the vault's isolated database (git-style).
#[tauri::command]
pub async fn commit_essay(
    state: State<'_, AppState>,
    vault_id: String,
    chamber_id: String,
    essay_id: String,
    message: String,
    body: String,
) -> Result<EssayVersionDto, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let student = db.ensure_student(&chamber_id, "Student").await.map_err(|e| e.to_string())?;
    let version =
        db.commit_essay(&essay_id, &student, &message, &body).await.map_err(|e| e.to_string())?;
    Ok(version.into())
}

/// Full revision history for an essay, oldest first.
#[tauri::command]
pub async fn essay_history(
    state: State<'_, AppState>,
    vault_id: String,
    essay_id: String,
) -> Result<Vec<EssayVersionDto>, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let history = db.essay_history(&essay_id).await.map_err(|e| e.to_string())?;
    Ok(history.into_iter().map(EssayVersionDto::from).collect())
}

/// A named essay prompt slot, as offered in the Essay Version Control picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EssaySlotDto {
    pub id: String,
    pub label: String,
    pub word_limit: Option<i64>,
}

impl From<quill_storage::EssaySlot> for EssaySlotDto {
    fn from(s: quill_storage::EssaySlot) -> Self {
        Self { id: s.id, label: s.label, word_limit: s.word_limit }
    }
}

/// List the vault's custom essay slots (on top of the fixed defaults the UI
/// always offers).
#[tauri::command]
pub async fn get_essay_slots(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<EssaySlotDto>, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let slots = db.list_essay_slots().await.map_err(|e| e.to_string())?;
    Ok(slots.into_iter().map(EssaySlotDto::from).collect())
}

/// Add a custom essay slot (e.g. a school-specific supplement).
#[tauri::command]
pub async fn add_essay_slot(
    state: State<'_, AppState>,
    vault_id: String,
    label: String,
    word_limit: Option<i64>,
) -> Result<EssaySlotDto, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let slot = db.add_essay_slot(&label, word_limit).await.map_err(|e| e.to_string())?;
    Ok(slot.into())
}

/// Remove a custom essay slot from the picker (its committed revisions stay).
#[tauri::command]
pub async fn delete_essay_slot(
    state: State<'_, AppState>,
    vault_id: String,
    id: String,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    db.delete_essay_slot(&id).await.map_err(|e| e.to_string())
}

// --- BioSpark Forge ---------------------------------------------------------

/// A plugin offered in the store / manager, with its trust + install state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailablePlugin {
    pub manifest: PluginManifest,
    pub trust: TrustLevel,
    /// `"builtin"` (native, always available) or `"forge"` (installable store).
    pub source: String,
    pub installed: bool,
}

const CATALOG_KEY: &str = "forge:catalog";

async fn load_catalog(state: &AppState) -> Catalog {
    match state.core.get_setting(CATALOG_KEY).await {
        Ok(Some(json)) => Catalog::from_json(&json).unwrap_or_default(),
        _ => Catalog::new(),
    }
}

async fn save_catalog(state: &AppState, cat: &Catalog) -> Result<(), String> {
    let json = cat.to_json().map_err(|e| e.to_string())?;
    state.core.upsert_setting(CATALOG_KEY, &json).await.map_err(|e| e.to_string())
}

/// Resolve a plugin manifest across built-ins and the (installed or store) set.
async fn resolve_manifest(state: &AppState, id: &str) -> Option<PluginManifest> {
    if let Some(m) = state.forge.manifest(id) {
        return Some(m);
    }
    load_catalog(state).await.get(id).map(|s| s.manifest.clone())
}

/// List every plugin available to enable in a vault: native built-ins plus the
/// Forge store (with trust badges and install state).
#[tauri::command]
pub async fn list_available_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<AvailablePlugin>, String> {
    let catalog = load_catalog(&state).await;
    let mut out: Vec<AvailablePlugin> = state
        .forge
        .builtins()
        .iter()
        .map(|m| AvailablePlugin {
            manifest: m.clone(),
            trust: TrustLevel::Verified,
            source: "builtin".into(),
            installed: true,
        })
        .collect();
    for signed in state.forge.store() {
        out.push(AvailablePlugin {
            manifest: signed.manifest.clone(),
            trust: signed.trust_level(state.forge.trust()),
            source: "forge".into(),
            installed: catalog.contains(&signed.manifest.id),
        });
    }
    Ok(out)
}

/// Install a store plugin (verifies it exists and records it in the catalog).
#[tauri::command]
pub async fn install_plugin(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let signed = state.forge.store_plugin(&id).ok_or("plugin not found in store")?.clone();
    let mut catalog = load_catalog(&state).await;
    catalog.install(signed).map_err(|e| e.to_string())?;
    save_catalog(&state, &catalog).await
}

/// Uninstall a Forge plugin.
#[tauri::command]
pub async fn uninstall_plugin(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut catalog = load_catalog(&state).await;
    catalog.uninstall(&id).map_err(|e| e.to_string())?;
    save_catalog(&state, &catalog).await
}

/// Read a vault's plugin composition (enabled plugins + customization).
#[tauri::command]
pub async fn get_vault_composition(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<VaultComposition, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    state.core.vault_composition(vid).await.map_err(|e| e.to_string())
}

/// Persist a vault's plugin composition.
#[tauri::command]
pub async fn set_vault_composition(
    state: State<'_, AppState>,
    vault_id: String,
    composition: VaultComposition,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    state.core.set_vault_composition(vid, &composition).await.map_err(|e| e.to_string())
}

/// A generic plugin record returned to the declarative renderer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRecordDto {
    pub id: String,
    /// Parsed JSON payload (the record's fields).
    pub data: serde_json::Value,
}

async fn ensure_can_store(state: &AppState, plugin_id: &str) -> Result<(), String> {
    let manifest =
        resolve_manifest(state, plugin_id).await.ok_or("unknown plugin")?;
    if manifest.has_capability(Capability::StorePluginRecords) {
        Ok(())
    } else {
        Err(format!("plugin {plugin_id} lacks the store-records capability"))
    }
}

/// Add a record to a declarative plugin's collection (capability-gated).
#[tauri::command]
pub async fn plugin_record_add(
    state: State<'_, AppState>,
    vault_id: String,
    chamber_id: Option<String>,
    plugin_id: String,
    collection: String,
    data: serde_json::Value,
) -> Result<PluginRecordDto, String> {
    ensure_can_store(&state, &plugin_id).await?;
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let payload = serde_json::to_string(&data).map_err(|e| e.to_string())?;
    let rec = db
        .plugin_record_add(&plugin_id, &collection, chamber_id.as_deref(), &payload)
        .await
        .map_err(|e| e.to_string())?;
    Ok(PluginRecordDto { id: rec.id, data })
}

/// List a declarative plugin collection's records (capability-gated).
#[tauri::command]
pub async fn plugin_record_list(
    state: State<'_, AppState>,
    vault_id: String,
    chamber_id: Option<String>,
    plugin_id: String,
    collection: String,
) -> Result<Vec<PluginRecordDto>, String> {
    ensure_can_store(&state, &plugin_id).await?;
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let rows = db
        .plugin_record_list(&plugin_id, &collection, chamber_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| PluginRecordDto {
            id: r.id,
            data: serde_json::from_str(&r.data).unwrap_or(serde_json::Value::Null),
        })
        .collect())
}

/// Delete a declarative plugin record.
#[tauri::command]
pub async fn plugin_record_delete(
    state: State<'_, AppState>,
    vault_id: String,
    id: String,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    db.plugin_record_delete(&id).await.map_err(|e| e.to_string())
}

/// Add a milestone to a chamber's application timeline.
#[tauri::command]
pub async fn add_milestone(
    state: State<'_, AppState>,
    vault_id: String,
    chamber_id: String,
    title: String,
    due_at: Option<String>,
) -> Result<MilestoneDto, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let student = db.ensure_student(&chamber_id, "Student").await.map_err(|e| e.to_string())?;
    let m = db
        .add_milestone(&student, &title, due_at.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(m.into())
}

/// List a chamber's milestones, chronologically.
#[tauri::command]
pub async fn list_milestones(
    state: State<'_, AppState>,
    vault_id: String,
    chamber_id: String,
) -> Result<Vec<MilestoneDto>, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let student = db.ensure_student(&chamber_id, "Student").await.map_err(|e| e.to_string())?;
    let list = db.list_milestones(&student).await.map_err(|e| e.to_string())?;
    Ok(list.into_iter().map(MilestoneDto::from).collect())
}

/// Toggle a milestone's completion state.
#[tauri::command]
pub async fn set_milestone_done(
    state: State<'_, AppState>,
    vault_id: String,
    id: String,
    done: bool,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    db.set_milestone_done(&id, done).await.map_err(|e| e.to_string())
}

// --- Billing Ledger ---------------------------------------------------------

/// One entry in the hour-tracking ledger, as shown in the Billing panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingEntryDto {
    pub id: String,
    pub student_id: Option<String>,
    pub minutes: i64,
    pub description: String,
    pub billed_at: String,
}

impl From<quill_storage::BillingEntry> for BillingEntryDto {
    fn from(e: quill_storage::BillingEntry) -> Self {
        Self {
            id: e.id,
            student_id: e.student_id,
            minutes: e.minutes,
            description: e.description,
            billed_at: e.billed_at,
        }
    }
}

/// A vault's full billing ledger: the retainer, what's been used, what
/// remains, and the entry history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingLedgerDto {
    pub retainer_minutes: i64,
    pub used_minutes: i64,
    pub balance_minutes: i64,
    pub entries: Vec<BillingEntryDto>,
}

/// Read a vault's full billing ledger.
#[tauri::command]
pub async fn get_billing_ledger(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<BillingLedgerDto, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let retainer_minutes = db.retainer_minutes().await.map_err(|e| e.to_string())?;
    let used_minutes = db.billing_minutes_used().await.map_err(|e| e.to_string())?;
    let entries = db.list_billing_entries().await.map_err(|e| e.to_string())?;
    Ok(BillingLedgerDto {
        retainer_minutes,
        used_minutes,
        balance_minutes: retainer_minutes - used_minutes,
        entries: entries.into_iter().map(BillingEntryDto::from).collect(),
    })
}

/// Top up (or otherwise set) the vault's total purchased retainer.
#[tauri::command]
pub async fn set_retainer_minutes(
    state: State<'_, AppState>,
    vault_id: String,
    minutes: i64,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    db.set_retainer_minutes(minutes).await.map_err(|e| e.to_string())
}

/// Log time worked against a vault's retainer.
#[tauri::command]
pub async fn log_billing_entry(
    state: State<'_, AppState>,
    vault_id: String,
    student_id: Option<String>,
    minutes: i64,
    description: String,
    billed_at: String,
) -> Result<BillingEntryDto, String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    let entry = db
        .log_billing_entry(student_id.as_deref(), minutes, &description, &billed_at)
        .await
        .map_err(|e| e.to_string())?;
    Ok(entry.into())
}

/// Delete a billing ledger entry.
#[tauri::command]
pub async fn delete_billing_entry(
    state: State<'_, AppState>,
    vault_id: String,
    id: String,
) -> Result<(), String> {
    let vid = VaultId(uuid::Uuid::parse_str(&vault_id).map_err(|e| e.to_string())?);
    let db = state.vaults.vault(vid).await.map_err(|e| e.to_string())?;
    db.delete_billing_entry(&id).await.map_err(|e| e.to_string())
}

/// Ask Quantum Quill inside a chamber. Enforces per-chamber authorization before
/// dispatching to the Omni-Route router.
#[tauri::command]
pub async fn invoke_quantum_quill(
    state: State<'_, AppState>,
    chamber_id: String,
    prompt: String,
) -> Result<ChatReplyDto, String> {
    let vid = chamber_vault_id(&chamber_id);
    let auth = state.core.ai_authorization(vid).await.map_err(|e| e.to_string())?;
    let reply = state
        .agent
        .respond_in_chamber(&auth, vec![Message::user(prompt)], 1024)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ChatReplyDto {
        text: reply.text,
        provider: reply.provider.to_string(),
        model: reply.model,
    })
}
