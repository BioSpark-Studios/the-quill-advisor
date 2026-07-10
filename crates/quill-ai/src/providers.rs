//! Concrete [`LlmProvider`] implementations, one per backend.
//!
//! Each builds the backend's native request shape from the provider-agnostic
//! [`CompletionRequest`], calls it with a shared `reqwest::Client` (rustls,
//! honours `HTTPS_PROXY`), and parses the first text span out of the response.
//! Defaults follow current best practice — the Anthropic default is
//! `claude-opus-4-8`, and `temperature` is never sent (it is rejected on
//! Opus 4.8 / 4.7 / Sonnet 5).

use crate::error::{AiError, Result};
use crate::provider::{Completion, CompletionRequest, LlmProvider, ProviderKind, Role};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Default model per provider when the caller doesn't override it.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-4-8";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-pro";
pub const DEFAULT_OLLAMA_MODEL: &str = "llama3.1";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn read_error(provider: ProviderKind, resp: reqwest::Response) -> AiError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    AiError::Status {
        provider: provider.to_string(),
        status,
        body: truncate(&body, 500),
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

// ---------------------------------------------------------------------------
// Anthropic
// ---------------------------------------------------------------------------

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Build with an API key, using the default model and endpoint.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: client(),
            api_key: api_key.into(),
            model: DEFAULT_ANTHROPIC_MODEL.to_string(),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    /// Override the model id (builder style).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }
    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, req: &CompletionRequest) -> Result<Completion> {
        // Anthropic keeps `system` separate and only accepts user/assistant turns.
        let msgs: Vec<Value> = req
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| json!({"role": role_str(m.role), "content": m.content}))
            .collect();
        let mut body = json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "messages": msgs,
        });
        if let Some(sys) = &req.system {
            body["system"] = json!(sys);
        }

        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(read_error(self.kind(), resp).await);
        }
        let v: Value = resp
            .json()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        let text = v["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find(|b| b["type"] == "text"))
            .and_then(|b| b["text"].as_str())
            .ok_or_else(|| AiError::Parse {
                provider: self.kind().to_string(),
                detail: "no text block in content".into(),
            })?;
        Ok(Completion {
            text: text.to_string(),
            provider: self.kind(),
            model: self.model.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// OpenAI
// ---------------------------------------------------------------------------

/// OpenAI Chat Completions provider.
pub struct OpenAiProvider {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: client(),
            api_key: api_key.into(),
            model: DEFAULT_OPENAI_MODEL.to_string(),
            base_url: "https://api.openai.com".to_string(),
        }
    }
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }
    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, req: &CompletionRequest) -> Result<Completion> {
        // OpenAI carries the system prompt as a leading system message.
        let mut msgs: Vec<Value> = Vec::new();
        if let Some(sys) = &req.system {
            msgs.push(json!({"role": "system", "content": sys}));
        }
        msgs.extend(
            req.messages
                .iter()
                .map(|m| json!({"role": role_str(m.role), "content": m.content})),
        );
        let body = json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "messages": msgs,
        });
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(read_error(self.kind(), resp).await);
        }
        let v: Value = resp
            .json()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::Parse {
                provider: self.kind().to_string(),
                detail: "no choices[0].message.content".into(),
            })?;
        Ok(Completion {
            text: text.to_string(),
            provider: self.kind(),
            model: self.model.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Gemini
// ---------------------------------------------------------------------------

/// Google Gemini provider.
pub struct GeminiProvider {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: client(),
            api_key: api_key.into(),
            model: DEFAULT_GEMINI_MODEL.to_string(),
            base_url: "https://generativelanguage.googleapis.com".to_string(),
        }
    }
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Gemini
    }
    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, req: &CompletionRequest) -> Result<Completion> {
        let contents: Vec<Value> = req
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = if m.role == Role::Assistant {
                    "model"
                } else {
                    "user"
                };
                json!({"role": role, "parts": [{"text": m.content}]})
            })
            .collect();
        let mut body = json!({ "contents": contents });
        if let Some(sys) = &req.system {
            body["system_instruction"] = json!({"parts": [{"text": sys}]});
        }
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let resp = self
            .http
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(read_error(self.kind(), resp).await);
        }
        let v: Value = resp
            .json()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        let text = v["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::Parse {
                provider: self.kind().to_string(),
                detail: "no candidates[0].content.parts[0].text".into(),
            })?;
        Ok(Completion {
            text: text.to_string(),
            provider: self.kind(),
            model: self.model.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Ollama (local, no key)
// ---------------------------------------------------------------------------

/// Local Ollama provider — needs no API key, so it's the default smoke-test path.
pub struct OllamaProvider {
    http: reqwest::Client,
    model: String,
    base_url: String,
}

impl OllamaProvider {
    /// Build against the default local endpoint (`http://localhost:11434`).
    pub fn new() -> Self {
        Self {
            http: client(),
            model: DEFAULT_OLLAMA_MODEL.to_string(),
            base_url: "http://localhost:11434".to_string(),
        }
    }
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
    /// Point at a non-default Ollama host.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }
    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, req: &CompletionRequest) -> Result<Completion> {
        let mut msgs: Vec<Value> = Vec::new();
        if let Some(sys) = &req.system {
            msgs.push(json!({"role": "system", "content": sys}));
        }
        msgs.extend(
            req.messages
                .iter()
                .map(|m| json!({"role": role_str(m.role), "content": m.content})),
        );
        let body = json!({ "model": self.model, "messages": msgs, "stream": false });
        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(read_error(self.kind(), resp).await);
        }
        let v: Value = resp
            .json()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        let text = v["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::Parse {
                provider: self.kind().to_string(),
                detail: "no message.content".into(),
            })?;
        Ok(Completion {
            text: text.to_string(),
            provider: self.kind(),
            model: self.model.clone(),
        })
    }
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}
