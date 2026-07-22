//! # quill-ai
//!
//! The AI layer for The Quill Advisor:
//!
//! * [`provider`] — the `LlmProvider` contract and provider-agnostic
//!   request/response types.
//! * [`providers`] — concrete backends: OpenAI, Anthropic, Gemini, Ollama.
//! * [`router`] — **Omni-Route**, the interchangeable-LLM router with fallback.
//! * [`agent`] — the **Quantum Quill** agent, gated by per-chamber AI
//!   authorization from `quill-core`.
//!
//! Network calls are only made by the concrete providers; the router, agent, and
//! authorization logic are all exercised in unit tests with an in-process mock
//! provider (the dependency-injection pattern from the architecture docs).

#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod agent;
pub mod error;
pub mod provider;
pub mod providers;
pub mod router;

pub use agent::{QuantumQuillAgent, QuillConfig};
pub use error::{AiError, Result};
pub use provider::{Completion, CompletionRequest, LlmProvider, Message, ProviderKind, Role};
pub use providers::{AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider};
pub use router::Router;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quill_core::authz::ChamberAiAuthorization;
    use quill_core::VaultId;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A mock provider that either returns canned text or always fails, and
    /// counts how many times it was called — for testing routing + fallback.
    struct MockProvider {
        kind: ProviderKind,
        reply: Option<String>,
        calls: Arc<AtomicUsize>,
    }

    impl MockProvider {
        fn ok(kind: ProviderKind, reply: &str) -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Arc::new(Self {
                    kind,
                    reply: Some(reply.to_string()),
                    calls: calls.clone(),
                }),
                calls,
            )
        }
        fn failing(kind: ProviderKind) -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Arc::new(Self {
                    kind,
                    reply: None,
                    calls: calls.clone(),
                }),
                calls,
            )
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn kind(&self) -> ProviderKind {
            self.kind
        }
        fn model(&self) -> &str {
            "mock"
        }
        async fn complete(&self, _req: &CompletionRequest) -> Result<Completion> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match &self.reply {
                Some(text) => Ok(Completion {
                    text: text.clone(),
                    provider: self.kind,
                    model: "mock".into(),
                }),
                None => Err(AiError::Http("mock failure".into())),
            }
        }
    }

    #[tokio::test]
    async fn router_uses_primary_when_it_succeeds() {
        let (primary, primary_calls) = MockProvider::ok(ProviderKind::Ollama, "local answer");
        let (fallback, fallback_calls) = MockProvider::ok(ProviderKind::Anthropic, "cloud answer");
        let router = Router::new(vec![primary, fallback]);

        let out = router
            .complete(&CompletionRequest::single_turn("sys", "hi"))
            .await
            .unwrap();
        assert_eq!(out.text, "local answer");
        assert_eq!(out.provider, ProviderKind::Ollama);
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        // Fallback never invoked when the primary succeeds.
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn router_falls_back_on_primary_failure() {
        let (primary, primary_calls) = MockProvider::failing(ProviderKind::Ollama);
        let (fallback, fallback_calls) = MockProvider::ok(ProviderKind::Anthropic, "cloud answer");
        let router = Router::new(vec![primary, fallback]);

        let out = router
            .complete(&CompletionRequest::single_turn("sys", "hi"))
            .await
            .unwrap();
        assert_eq!(out.text, "cloud answer");
        assert_eq!(out.provider, ProviderKind::Anthropic);
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn router_errors_when_all_fail() {
        let (a, _) = MockProvider::failing(ProviderKind::Ollama);
        let (b, _) = MockProvider::failing(ProviderKind::OpenAi);
        let router = Router::new(vec![a, b]);
        let err = router
            .complete(&CompletionRequest::single_turn("s", "u"))
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::AllProvidersFailed(_)));
    }

    #[tokio::test]
    async fn empty_router_errors() {
        let err = Router::empty()
            .complete(&CompletionRequest::single_turn("s", "u"))
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::NoProviders));
    }

    #[tokio::test]
    async fn quill_blocked_in_unauthorized_chamber() {
        let (mock, calls) = MockProvider::ok(ProviderKind::Ollama, "draft");
        let router = Arc::new(Router::new(vec![mock]));
        let agent = QuantumQuillAgent::new(QuillConfig::quantum_quill(), router);

        let chamber = VaultId::new();
        let mut auth = ChamberAiAuthorization::disabled(chamber);

        // Chamber AI is off → the agent must refuse and never call the provider.
        let err = agent
            .respond_in_chamber(&auth, vec![Message::user("help")], 256)
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::Core(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        // Counselor enables the chamber and authorizes this agent → it responds.
        auth.set_enabled(true);
        auth.authorize(agent.id());
        let out = agent
            .respond_in_chamber(&auth, vec![Message::user("help")], 256)
            .await
            .unwrap();
        assert_eq!(out.text, "draft");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn quill_config_serde_and_system_prompt() {
        let cfg = QuillConfig::quantum_quill();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: QuillConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.identity.name, "Quantum Quill");
        assert!(cfg.system_prompt().contains("Quantum Quill"));
    }
}
