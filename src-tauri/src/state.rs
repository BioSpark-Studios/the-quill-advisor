//! Application state: the global core database, the Master vault id, and the
//! Quantum Quill agent (with its Omni-Route router built from environment keys).

use anyhow::Result;
use quill_ai::providers::{AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider};
use quill_ai::{LlmProvider, QuantumQuillAgent, QuillConfig, Router};
use quill_core::VaultId;
use quill_storage::CoreDb;
use std::path::Path;
use std::sync::Arc;

/// Shared, cloneable application state managed by Tauri.
pub struct AppState {
    /// The global Level-0 store.
    pub core: CoreDb,
    /// The Master vault node id.
    pub master_id: VaultId,
    /// The Quantum Quill agent bound to the Omni-Route router.
    pub agent: Arc<QuantumQuillAgent>,
}

impl AppState {
    /// Initialize the backend: open `core_db`, ensure the Master vault exists,
    /// and build the AI router + agent.
    ///
    /// # Errors
    /// Fails if the database cannot be opened or migrated.
    pub async fn init(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let core = CoreDb::open(data_dir.join("core.db")).await?;
        let master = core.ensure_master("Master Vault").await?;

        let router = Arc::new(build_router());
        let agent = Arc::new(QuantumQuillAgent::new(QuillConfig::quantum_quill(), router));

        Ok(Self { core, master_id: master.node.id, agent })
    }
}

/// Build the Omni-Route provider chain from environment variables. Cloud
/// providers (when their key is set) come first; local Ollama is always appended
/// as an offline fallback so the app works with no keys at all.
fn build_router() -> Router {
    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            providers.push(Arc::new(AnthropicProvider::new(key)));
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            providers.push(Arc::new(OpenAiProvider::new(key)));
        }
    }
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.is_empty() {
            providers.push(Arc::new(GeminiProvider::new(key)));
        }
    }
    let ollama = match std::env::var("OLLAMA_BASE_URL") {
        Ok(url) if !url.is_empty() => OllamaProvider::new().with_base_url(url),
        _ => OllamaProvider::new(),
    };
    providers.push(Arc::new(ollama));
    Router::new(providers)
}
