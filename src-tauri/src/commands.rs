//! Tauri IPC commands. The command names and argument shapes mirror
//! `ui/src/lib/ipc.ts`. Each maps UI actions onto the real `quill-*` crates.

use crate::state::AppState;
use quill_ai::Message;
use quill_core::authz::ChamberAiAuthorization;
use quill_core::vault::VaultNode;
use quill_core::VaultId;
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
    pub hour_balance: i64,
    pub last_activity: String,
}

/// Card metadata persisted alongside a vault node (settings key `card:{id}`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CardMeta {
    stage: String,
    students: i64,
    chambers: i64,
    hour_balance: i64,
}

/// The Quill chat reply returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatReplyDto {
    pub text: String,
    pub provider: String,
    pub model: String,
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
        cards.push(VaultCardDto {
            id: rec.node.id.to_string(),
            name: rec.node.name,
            stage: if meta.stage.is_empty() { "active".into() } else { meta.stage },
            students: meta.students,
            chambers: meta.chambers,
            hour_balance: meta.hour_balance,
            last_activity: rec.created_at.format("%Y-%m-%d").to_string(),
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
    Ok(VaultCardDto {
        id: node.id.to_string(),
        name,
        stage,
        students: 0,
        chambers: 0,
        hour_balance: 0,
        last_activity: "just now".into(),
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
