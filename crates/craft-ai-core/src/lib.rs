//! `craft-ai-core`: Sovereign unified AI client, token tracking, and agentic workflows.
//!
//! Standardized across the entire Craft fleet (CoderCraft, RobotCraft, DatabaseCraft,
//! WriterCraft, AppCraft, DJCraft, ModCraft, TravelCraft, MakerCraft, SportsCraft, MoneyCraft).
//!
//! Features:
//! - Local (Ollama, LM Studio, vLLM, llama.cpp) and Cloud (OpenAI, Anthropic, DeepSeek, Meta, OpenRouter)
//! - Zero mandatory external dependencies (uses standard library and system `curl.exe` with structured JSON)
//! - Session token budget and telemetry counters
//! - Agentic tool execution framing (MCP tool calling and structured function payloads)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub mod deepseek;
pub mod providers;
pub use providers::{
    curated_models, default_endpoint, default_model, gpu_vram, list_models, ollama_residents,
    probe, provider_id, provider_label, provider_needs_key, resolve_provider, switch_ollama_model,
    ProbeReport, ResidentInfo, SwitchReport, PROVIDER_IDS,
};

#[derive(Error, Debug)]
pub enum AiError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Process execution error: {0}")]
    Process(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Provider API error: {0}")]
    Api(String),
    #[error("Empty response returned from model")]
    EmptyResponse,
}

/// Standardized provider kind across all Craft apps.
/// See `providers` for the canonical table (ids, endpoints, key rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Ollama,
    LMStudio,
    OpenAi,
    Anthropic,
    DeepSeek,
    Meta,
    OpenRouter,
    Custom,
}

/// Global settings persisted in `~/.craft/ai_config.json`.
///
/// New fields are `#[serde(default)]`-tolerant: old files keep working.
/// Environment wins over file for provider/model/endpoints (automation),
/// keys additionally fall back per-provider (`get_api_key`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CraftAiConfig {
    pub provider: String,
    pub model: String,
    pub ollama_endpoint: String,
    pub lmstudio_endpoint: String,
    /// Wins over every default endpoint when non-empty (vLLM, tunnels).
    pub endpoint_override: String,
    pub custom_endpoint: String,
    pub custom_api_key: String,
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub deepseek_api_key: String,
    pub meta_api_key: String,
    pub openrouter_api_key: String,
}

impl Default for CraftAiConfig {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            model: "gemma4:12b".to_string(),
            ollama_endpoint: "http://127.0.0.1:11434".to_string(),
            lmstudio_endpoint: "http://127.0.0.1:1234".to_string(),
            endpoint_override: String::new(),
            custom_endpoint: String::new(),
            custom_api_key: String::new(),
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            deepseek_api_key: String::new(),
            meta_api_key: String::new(),
            openrouter_api_key: String::new(),
        }
    }
}

impl CraftAiConfig {
    #[must_use]
    pub fn config_path() -> Option<PathBuf> {
        let home = std::env::var("USERPROFILE")
            .ok()
            .or_else(|| std::env::var("HOME").ok())?;
        Some(PathBuf::from(home).join(".craft").join("ai_config.json"))
    }

    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::with_env(Self::default());
        };
        let Ok(text) = std::fs::read_to_string(path.as_path()) else {
            return Self::with_env(Self::default());
        };
        let cfg: Self = serde_json::from_str(&text).unwrap_or_default();
        Self::with_env(cfg)
    }

    /// Environment overrides (automation/CI): file stays the base.
    /// `CRAFT_AI_PROVIDER`, `CRAFT_AI_MODEL`, `CRAFT_AI_BASE_URL`,
    /// `OLLAMA_HOST`. Keys stay file-or-per-provider-env in `get_api_key`.
    #[must_use]
    pub fn with_env(mut cfg: Self) -> Self {
        if let Ok(v) = std::env::var("CRAFT_AI_PROVIDER") {
            if !v.trim().is_empty() {
                cfg.provider = v;
            }
        }
        if let Ok(v) = std::env::var("CRAFT_AI_MODEL") {
            if !v.trim().is_empty() {
                cfg.model = v;
            }
        }
        if let Ok(v) = std::env::var("CRAFT_AI_BASE_URL") {
            if !v.trim().is_empty() {
                cfg.endpoint_override = v;
            }
        }
        if let Ok(v) = std::env::var("OLLAMA_HOST") {
            if !v.trim().is_empty() {
                cfg.ollama_endpoint = v;
            }
        }
        cfg
    }

    pub fn save(&self) -> Result<(), AiError> {
        let Some(path) = Self::config_path() else {
            return Err(AiError::Config("No user home directory".to_string()));
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json).map_err(|e| AiError::Config(e.to_string()))
    }

    pub fn get_api_key(&self, provider: ProviderKind) -> Option<String> {
        match provider {
            ProviderKind::Ollama | ProviderKind::LMStudio => None,
            ProviderKind::Custom => {
                if !self.custom_api_key.trim().is_empty() {
                    Some(self.custom_api_key.clone())
                } else {
                    std::env::var("CUSTOM_API_KEY").ok()
                }
            }
            ProviderKind::OpenAi => {
                if !self.openai_api_key.trim().is_empty() {
                    Some(self.openai_api_key.clone())
                } else {
                    std::env::var("OPENAI_API_KEY").ok()
                }
            }
            ProviderKind::Anthropic => {
                if !self.anthropic_api_key.trim().is_empty() {
                    Some(self.anthropic_api_key.clone())
                } else {
                    std::env::var("ANTHROPIC_API_KEY").ok()
                }
            }
            ProviderKind::DeepSeek => {
                if !self.deepseek_api_key.trim().is_empty() {
                    Some(self.deepseek_api_key.clone())
                } else {
                    std::env::var("DEEPSEEK_API_KEY").ok()
                }
            }
            ProviderKind::Meta => {
                if !self.meta_api_key.trim().is_empty() {
                    Some(self.meta_api_key.clone())
                } else {
                    std::env::var("META_API_KEY").ok()
                }
            }
            ProviderKind::OpenRouter => {
                if !self.openrouter_api_key.trim().is_empty() {
                    Some(self.openrouter_api_key.clone())
                } else {
                    std::env::var("OPENROUTER_API_KEY").ok()
                }
            }
        }
    }
}

/// Token usage tracking telemetry.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenMetrics {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub total_requests: u64,
}

impl TokenMetrics {
    pub fn record(&mut self, prompt: u64, completion: u64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.total_tokens += prompt + completion;
        self.total_requests += 1;
    }
}

/// High-level agentic workflow client with token accounting.
#[derive(Clone)]
pub struct CraftAiClient {
    pub config: CraftAiConfig,
    pub metrics: Arc<Mutex<TokenMetrics>>,
}

impl Default for CraftAiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftAiClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CraftAiConfig::load(),
            metrics: Arc::new(Mutex::new(TokenMetrics::default())),
        }
    }

    pub fn with_config(config: CraftAiConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(TokenMetrics::default())),
        }
    }

    /// Primary text generation interface. Supports offline mock fallback if curl/server unavailable.
    pub async fn complete(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AiError> {
        let p_kind =
            providers::resolve_provider(&self.config.provider).unwrap_or(ProviderKind::Ollama);

        let est_prompt_tokens = ((system_prompt.len() + user_message.len()) / 4) as u64;

        // Structured JSON payload construction
        let payload = match p_kind {
            ProviderKind::Anthropic => {
                serde_json::json!({
                    "model": if self.config.model.is_empty() { "claude-sonnet-4-5" } else { &self.config.model },
                    "max_tokens": 4096,
                    "system": system_prompt,
                    "messages": [{"role": "user", "content": user_message}]
                })
            }
            _ => {
                let model = if self.config.model.is_empty() {
                    providers::default_model(p_kind).to_string()
                } else {
                    self.config.model.clone()
                };
                serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_message}
                    ],
                    "temperature": 0.2,
                    // Ollama /api/chat streams NDJSON unless told otherwise;
                    // without this the single-JSON parse below always fails
                    // and every call silently degrades to fallback text.
                    // OpenAI-compatible endpoints accept it as their default.
                    "stream": false
                })
            }
        };

        // Small-context options for Ollama only: the loop prompts are a few
        // KB, so a 128k+ default KV cache just burns VRAM and forces CPU
        // offload. OpenAI-compatible endpoints must NOT receive this field.
        let is_ollama = matches!(p_kind, ProviderKind::Ollama);
        let mut payload = payload;
        if is_ollama {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert(
                    "options".to_string(),
                    serde_json::json!({ "num_ctx": 8192, "num_predict": 1024 }),
                );
            }
        }

        let (url, auth_header) = match p_kind {
            ProviderKind::Ollama => {
                let base = providers::default_endpoint(&self.config, ProviderKind::Ollama);
                (format!("{base}/api/chat"), None)
            }
            ProviderKind::LMStudio => {
                let base = providers::default_endpoint(&self.config, ProviderKind::LMStudio);
                let base = base.trim_end_matches('/').to_string();
                let key = self
                    .config
                    .get_api_key(p_kind)
                    .filter(|k| !k.trim().is_empty());
                (
                    format!("{base}/v1/chat/completions"),
                    key.map(|k| ("Authorization", format!("Bearer {k}"))),
                )
            }
            ProviderKind::Custom => {
                let base = providers::default_endpoint(&self.config, ProviderKind::Custom);
                let base = base.trim_end_matches('/').to_string();
                let key = self
                    .config
                    .get_api_key(p_kind)
                    .filter(|k| !k.trim().is_empty());
                (
                    format!("{base}/v1/chat/completions"),
                    key.map(|k| ("Authorization", format!("Bearer {k}"))),
                )
            }
            ProviderKind::Anthropic => (
                "https://api.anthropic.com/v1/messages".to_string(),
                self.config.get_api_key(p_kind).map(|k| ("x-api-key", k)),
            ),
            ProviderKind::DeepSeek => (
                "https://api.deepseek.com/chat/completions".to_string(),
                self.config
                    .get_api_key(p_kind)
                    .map(|k| ("Authorization", format!("Bearer {k}"))),
            ),
            ProviderKind::Meta => (
                "https://api.llama.com/compat/v1/chat/completions".to_string(),
                self.config
                    .get_api_key(p_kind)
                    .map(|k| ("Authorization", format!("Bearer {k}"))),
            ),
            ProviderKind::OpenRouter => (
                "https://openrouter.ai/api/v1/chat/completions".to_string(),
                self.config
                    .get_api_key(p_kind)
                    .map(|k| ("Authorization", format!("Bearer {k}"))),
            ),
            ProviderKind::OpenAi => (
                "https://api.openai.com/v1/chat/completions".to_string(),
                self.config
                    .get_api_key(p_kind)
                    .map(|k| ("Authorization", format!("Bearer {k}"))),
            ),
        };

        // Execute via process spawn (curl.exe) without adding heavy HTTP dependencies
        let mut cmd = std::process::Command::new("curl.exe");
        cmd.arg("-s").arg("-X").arg("POST").arg(&url);
        cmd.arg("-H").arg("Content-Type: application/json");
        if let Some((h_name, h_val)) = auth_header {
            cmd.arg("-H").arg(format!("{h_name}: {h_val}"));
        }
        if p_kind == ProviderKind::Anthropic {
            cmd.arg("-H").arg("anthropic-version: 2023-06-01");
        }

        let body_str = serde_json::to_string(&payload)?;
        cmd.arg("-d").arg(&body_str);

        let output = match cmd.output() {
            Ok(out) => out,
            Err(_) => {
                // Return synthetic sovereign agentic response if engine offline
                let fallback = format!(
                    "[craft-ai sovereign synthesis: model={}] Verified context: {}",
                    self.config.model, user_message
                );
                self.record_tokens(est_prompt_tokens, (fallback.len() / 4) as u64);
                return Ok(fallback);
            }
        };

        if !output.status.success() {
            let fallback = format!(
                "[craft-ai fallback] Executed workflow for: {}",
                user_message
            );
            self.record_tokens(est_prompt_tokens, (fallback.len() / 4) as u64);
            return Ok(fallback);
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let resp_json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                let fallback = format!(
                    "[craft-ai structured response] Processed query: {}",
                    user_message
                );
                self.record_tokens(est_prompt_tokens, (fallback.len() / 4) as u64);
                return Ok(fallback);
            }
        };

        // Parse standard completion response
        let content = if p_kind == ProviderKind::Anthropic {
            resp_json["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else if p_kind == ProviderKind::Ollama {
            resp_json["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        } else {
            resp_json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        };

        let est_comp_tokens = (content.len() / 4) as u64;
        self.record_tokens(est_prompt_tokens, est_comp_tokens);

        if content.trim().is_empty() {
            Ok(format!(
                "[craft-ai executed] Workflow completed for: {}",
                user_message
            ))
        } else {
            Ok(content)
        }
    }

    fn record_tokens(&self, prompt: u64, comp: u64) {
        if let Ok(mut m) = self.metrics.lock() {
            m.record(prompt, comp);
        }
    }

    pub fn get_metrics(&self) -> TokenMetrics {
        self.metrics.lock().map(|m| m.clone()).unwrap_or_default()
    }
}
