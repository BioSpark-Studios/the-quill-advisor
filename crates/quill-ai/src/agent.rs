//! The Quantum Quill agent — the conversational drafting assistant.
//!
//! Config-driven (identity + behavior + suggestions), it composes a system
//! prompt and talks to the vault's [`Router`]. Crucially, when it acts inside a
//! student chamber it must first pass the counselor's per-chamber AI
//! authorization gate ([`ChamberAiAuthorization`]) — this is the tie between the
//! AI layer and the vault security model. The config shape mirrors the
//! `QuillConfig` in `docs/architecture/quantum-nexus-core-framework.md`.

use crate::error::Result;
use crate::provider::{Completion, CompletionRequest, Message};
use crate::router::Router;
use quill_core::authz::ChamberAiAuthorization;
use quill_core::ids::AgentId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Who the agent is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Display name, e.g. "Quantum Quill".
    pub name: String,
    /// Role summary, e.g. "Conversational drafting assistant".
    pub role: String,
}

/// How the agent behaves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    /// Master enable switch.
    pub enabled: bool,
    /// Suggestion chips offered in the chat UI on first open.
    #[serde(rename = "initialSuggestions", default)]
    pub initial_suggestions: Vec<String>,
}

/// Full agent configuration, deserialized from a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuillConfig {
    /// The agent's stable id (matches the id used in per-chamber authorization).
    pub agent_id: AgentId,
    /// Identity block.
    pub identity: Identity,
    /// Behavior block.
    pub behavior: Behavior,
    /// The base instruction that grounds the agent's persona.
    #[serde(default = "default_instruction")]
    pub instruction: String,
}

fn default_instruction() -> String {
    "You are a supportive, precise college-admissions writing coach. Help students \
     draft and refine application materials in their own authentic voice. Be concise, \
     concrete, and encouraging."
        .to_string()
}

impl QuillConfig {
    /// A sensible default Quantum Quill configuration.
    #[must_use]
    pub fn quantum_quill() -> Self {
        Self {
            agent_id: AgentId::new(),
            identity: Identity {
                name: "Quantum Quill".to_string(),
                role: "Conversational drafting assistant".to_string(),
            },
            behavior: Behavior {
                enabled: true,
                initial_suggestions: vec![
                    "Brainstorm essay topics".to_string(),
                    "Tighten my introduction".to_string(),
                    "Check my authentic voice".to_string(),
                ],
            },
            instruction: default_instruction(),
        }
    }

    /// The system prompt derived from identity + instruction.
    #[must_use]
    pub fn system_prompt(&self) -> String {
        format!(
            "You are {} — {}.\n\n{}",
            self.identity.name, self.identity.role, self.instruction
        )
    }
}

/// The Quantum Quill agent, bound to a router.
pub struct QuantumQuillAgent {
    config: Arc<QuillConfig>,
    router: Arc<Router>,
}

impl QuantumQuillAgent {
    /// Build the agent from a config and router.
    #[must_use]
    pub fn new(config: QuillConfig, router: Arc<Router>) -> Self {
        Self {
            config: Arc::new(config),
            router,
        }
    }

    /// The agent's id (for authorization checks).
    #[must_use]
    pub fn id(&self) -> AgentId {
        self.config.agent_id
    }

    /// The configuration (shared).
    #[must_use]
    pub fn config(&self) -> Arc<QuillConfig> {
        Arc::clone(&self.config)
    }

    /// Respond to a conversation, prepending the agent's system prompt.
    ///
    /// # Errors
    /// Propagates router/provider errors.
    pub async fn respond(&self, history: Vec<Message>, max_tokens: u32) -> Result<Completion> {
        let req = CompletionRequest {
            system: Some(self.config.system_prompt()),
            messages: history,
            max_tokens,
        };
        self.router.complete(&req).await
    }

    /// Respond **inside a chamber**, first enforcing the counselor's per-chamber
    /// AI authorization gate. This is the mandatory entry point for any AI use
    /// that touches student data.
    ///
    /// # Errors
    /// [`crate::error::AiError::Core`] wrapping
    /// [`quill_core::CoreError::AgentNotAuthorized`] if the chamber has not
    /// enabled this agent; otherwise propagates router errors.
    pub async fn respond_in_chamber(
        &self,
        auth: &ChamberAiAuthorization,
        history: Vec<Message>,
        max_tokens: u32,
    ) -> Result<Completion> {
        auth.ensure_authorized(self.id())?;
        self.respond(history, max_tokens).await
    }
}
