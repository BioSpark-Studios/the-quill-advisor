//! Omni-Route: the interchangeable-LLM router.
//!
//! Holds an ordered list of providers and dispatches a [`CompletionRequest`] to
//! the primary, falling back to the next on failure. This is what lets a vault
//! prefer local Ollama for privacy but fall back to a cloud provider, or vice
//! versa, without any caller changes.

use crate::error::{AiError, Result};
use crate::provider::{Completion, CompletionRequest, LlmProvider, ProviderKind};
use std::sync::Arc;

/// A provider chain tried in order until one succeeds.
#[derive(Clone)]
pub struct Router {
    providers: Vec<Arc<dyn LlmProvider>>,
}

impl Router {
    /// Build a router from an ordered provider list (primary first).
    #[must_use]
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Self {
        Self { providers }
    }

    /// Build an empty router; add providers with [`Router::push`].
    #[must_use]
    pub fn empty() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Append a provider to the fallback chain.
    #[must_use]
    pub fn push(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// The provider kinds in priority order.
    #[must_use]
    pub fn chain(&self) -> Vec<ProviderKind> {
        self.providers.iter().map(|p| p.kind()).collect()
    }

    /// Route a completion, trying each provider in order until one succeeds.
    ///
    /// # Errors
    /// [`AiError::NoProviders`] if the chain is empty, or
    /// [`AiError::AllProvidersFailed`] if every provider errored.
    pub async fn complete(&self, req: &CompletionRequest) -> Result<Completion> {
        if self.providers.is_empty() {
            return Err(AiError::NoProviders);
        }
        let mut last_err: Option<AiError> = None;
        for provider in &self.providers {
            match provider.complete(req).await {
                Ok(completion) => return Ok(completion),
                Err(e) => {
                    tracing::warn!(provider = %provider.kind(), error = %e, "provider failed; falling back");
                    last_err = Some(e);
                }
            }
        }
        Err(AiError::AllProvidersFailed(
            last_err
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown".into()),
        ))
    }
}
