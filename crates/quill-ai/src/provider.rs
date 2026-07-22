//! The `LlmProvider` contract and the request/response types shared by every
//! provider in the Omni-Route router.

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The interchangeable LLM backends the router can dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// OpenAI Chat Completions.
    OpenAi,
    /// Anthropic Messages API.
    Anthropic,
    /// Google Gemini `generateContent`.
    Gemini,
    /// A local Ollama server (no API key required).
    Ollama,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Gemini => "gemini",
            ProviderKind::Ollama => "ollama",
        };
        f.write_str(s)
    }
}

/// The author of a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A system/developer instruction.
    System,
    /// The end user (or counselor driving the agent).
    User,
    /// The assistant's prior reply.
    Assistant,
}

/// A single chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Who wrote it.
    pub role: Role,
    /// The text content.
    pub content: String,
}

impl Message {
    /// Build a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Build an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    /// Build a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// A provider-agnostic completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// An optional top-level system prompt (kept separate from `messages` so
    /// providers that model it distinctly — Anthropic, Gemini — can place it
    /// correctly).
    pub system: Option<String>,
    /// The conversation so far.
    pub messages: Vec<Message>,
    /// Maximum output tokens.
    pub max_tokens: u32,
}

impl CompletionRequest {
    /// A single-turn request with a system prompt and one user message.
    pub fn single_turn(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            messages: vec![Message::user(user)],
            max_tokens: 1024,
        }
    }
}

/// A provider-agnostic completion result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// The generated text.
    pub text: String,
    /// Which provider served the request.
    pub provider: ProviderKind,
    /// The concrete model that produced the text.
    pub model: String,
}

/// The contract every backend implements. Object-safe (via `async_trait`) so the
/// router can hold `Arc<dyn LlmProvider>` and fall back across providers.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Which backend this is.
    fn kind(&self) -> ProviderKind;

    /// The model id this provider will call.
    fn model(&self) -> &str;

    /// Produce a completion for `request`.
    async fn complete(&self, request: &CompletionRequest) -> Result<Completion>;
}
