//! The Quill Advisor desktop shell (Tauri v2).

mod commands;
mod forge;
mod state;

use state::AppState;
use tauri::Manager;

/// Build and run the Tauri application. Named `run` so the mobile entry points
/// (`main.rs` and any future mobile shim) can share it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "quill_advisor_lib=info,quill_ai=info".into()),
        )
        .init();
    tracing::info!("Starting The Quill Advisor…");

    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            // Initialize the async backend before the window loads.
            let state = tauri::async_runtime::block_on(AppState::init(&data_dir))
                .map_err(|e| format!("backend init failed: {e}"))?;
            app.manage(state);
            tracing::info!("Backend initialized; UI and vault engine synchronized.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::get_vault_hierarchy,
            commands::create_vault,
            commands::move_vault,
            commands::chamber_ai_enabled,
            commands::set_chamber_ai,
            commands::invoke_quantum_quill,
            commands::commit_essay,
            commands::essay_history,
            commands::get_essay_slots,
            commands::add_essay_slot,
            commands::delete_essay_slot,
            commands::add_milestone,
            commands::list_milestones,
            commands::set_milestone_done,
            commands::get_billing_ledger,
            commands::set_retainer_minutes,
            commands::log_billing_entry,
            commands::delete_billing_entry,
            commands::list_available_plugins,
            commands::install_plugin,
            commands::uninstall_plugin,
            commands::get_vault_composition,
            commands::set_vault_composition,
            commands::plugin_record_add,
            commands::plugin_record_list,
            commands::plugin_record_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running The Quill Advisor");
}
