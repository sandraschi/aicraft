//! Fleet LLM provider table: Ollama, LM Studio, OpenAI, Anthropic,
//! DeepSeek, Meta, OpenRouter, Custom.
//!
//! This is the single place where "which providers exist, where do they
//! live, which need keys, how are models listed" is answered for every
//! Rust craft app. No per-app `llm.rs` may duplicate this table.
//!
//! Transport notes (all hard-won, do not "simplify"):
//! - Ollama `/api/chat` must send `"stream": false`, else the server returns
//!   NDJSON and single-JSON parsing silently degrades to fallback text.
//! - Thinking models must send `"think": false` (or a small `num_ctx`) or a
//!   short tool call drowns in a 60s reasoning trace.
//! - LM Studio is OpenAI-compatible (`/v1/chat/completions`, `/v1/models`)
//!   and needs NO key. Probing Ollama-first then LM Studio mirrors the
//!   Python fleet template (`templates/llm/llm_engine.py`).
//! - `switch_ollama_model` evicts every OTHER resident before warming the
//!   pick: saving a selection must switch VRAM, not just config.
//! - Anthropic exposes no list-models API: the dropdown is a curated list
//!   plus free-text entry. Reporting curated names as "live" without a key
//!   is a lie (fleet BUG-042): `list_models` errors when the key is absent.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

use crate::{AiError, CraftAiConfig, ProviderKind};

/// Canonical provider ids. These strings are the config-file and UI values.
pub const PROVIDER_IDS: &[&str] = &[
    "ollama",
    "lmstudio",
    "openai",
    "anthropic",
    "deepseek",
    "meta",
    "openrouter",
    "custom",
];

/// Resolve a user-facing name to a provider. Accepts the aliases the fleet
/// has historically used (`local`, `lms`, `cloud`).
#[must_use]
pub fn resolve_provider(name: &str) -> Option<ProviderKind> {
    match name.trim().to_lowercase().as_str() {
        "ollama" | "local" => Some(ProviderKind::Ollama),
        "lmstudio" | "lms" | "lm-studio" => Some(ProviderKind::LMStudio),
        "openai" | "cloud" | "gpt" => Some(ProviderKind::OpenAi),
        "anthropic" | "claude" => Some(ProviderKind::Anthropic),
        "deepseek" => Some(ProviderKind::DeepSeek),
        "meta" | "llama" => Some(ProviderKind::Meta),
        "openrouter" => Some(ProviderKind::OpenRouter),
        "custom" | "vllm" | "compat" => Some(ProviderKind::Custom),
        _ => None,
    }
}

/// Canonical id for a provider kind (config file + UI value).
#[must_use]
pub fn provider_id(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "ollama",
        ProviderKind::LMStudio => "lmstudio",
        ProviderKind::OpenAi => "openai",
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::DeepSeek => "deepseek",
        ProviderKind::Meta => "meta",
        ProviderKind::OpenRouter => "openrouter",
        ProviderKind::Custom => "custom",
    }
}

/// Human label for dropdowns.
#[must_use]
pub fn provider_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "Ollama (local)",
        ProviderKind::LMStudio => "LM Studio (local)",
        ProviderKind::OpenAi => "OpenAI (cloud)",
        ProviderKind::Anthropic => "Anthropic (cloud)",
        ProviderKind::DeepSeek => "DeepSeek (cloud)",
        ProviderKind::Meta => "Meta Llama API (cloud)",
        ProviderKind::OpenRouter => "OpenRouter (cloud)",
        ProviderKind::Custom => "Custom OpenAI-compatible",
    }
}

/// Whether the provider needs an API key for model listing and completion.
#[must_use]
pub fn provider_needs_key(kind: ProviderKind) -> bool {
    !matches!(kind, ProviderKind::Ollama | ProviderKind::LMStudio)
}

/// Default endpoint per provider. `endpoint_override` in config wins over all.
#[must_use]
pub fn default_endpoint(config: &CraftAiConfig, kind: ProviderKind) -> String {
    if !config.endpoint_override.trim().is_empty() {
        return config.endpoint_override.trim().to_string();
    }
    match kind {
        ProviderKind::Ollama => {
            if config.ollama_endpoint.trim().is_empty() {
                "http://127.0.0.1:11434".to_string()
            } else {
                config.ollama_endpoint.trim().to_string()
            }
        }
        ProviderKind::LMStudio => {
            if config.lmstudio_endpoint.trim().is_empty() {
                "http://127.0.0.1:1234".to_string()
            } else {
                config.lmstudio_endpoint.trim().to_string()
            }
        }
        ProviderKind::OpenAi => "https://api.openai.com".to_string(),
        ProviderKind::Anthropic => "https://api.anthropic.com".to_string(),
        ProviderKind::DeepSeek => "https://api.deepseek.com".to_string(),
        ProviderKind::Meta => "https://api.llama.com/compat/v1".to_string(),
        ProviderKind::OpenRouter => "https://openrouter.ai/api/v1".to_string(),
        ProviderKind::Custom => config.custom_endpoint.trim().to_string(),
    }
}

/// Sensible default model per provider. Empty means "user must pick".
#[must_use]
pub fn default_model(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "gemma4:12b",
        ProviderKind::LMStudio => "",
        ProviderKind::OpenAi => "gpt-4o-mini",
        ProviderKind::Anthropic => "claude-sonnet-4-5",
        ProviderKind::DeepSeek => "deepseek-flash",
        ProviderKind::Meta => "Llama-4-Maverick-17B-128E-Instruct-FP8",
        ProviderKind::OpenRouter => "",
        ProviderKind::Custom => "",
    }
}

/// Curated model ids for providers with no list API (Anthropic).
/// Update when providers ship new flagships. Free-text entry in Settings
/// always overrides this list.
#[must_use]
pub fn curated_models(kind: ProviderKind) -> Vec<String> {
    match kind {
        ProviderKind::Anthropic => vec![
            "claude-sonnet-4-5".to_string(),
            "claude-opus-4-1".to_string(),
            "claude-haiku-4-5".to_string(),
        ],
        _ => vec![],
    }
}

fn curl_text(args: &[&str], timeout_s: &str) -> Result<String, AiError> {
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sS").arg("--max-time").arg(timeout_s);
    for a in args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .map_err(|e| AiError::Process(format!("cannot run curl.exe: {e}")))?;
    if !output.status.success() {
        return Err(AiError::Process(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_post_json(url: &str, headers: &[(&str, &str)], body: &str) -> Result<String, AiError> {
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sS")
        .arg("--max-time")
        .arg("240")
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg("content-type: application/json")
        .arg("--data-binary")
        .arg(body);
    for (name, value) in headers {
        cmd.arg("-H").arg(format!("{name}: {value}"));
    }
    let output = cmd
        .output()
        .map_err(|e| AiError::Process(format!("cannot run curl.exe: {e}")))?;
    if !output.status.success() {
        return Err(AiError::Process(format!(
            "request failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Probe result for one provider: reachable + advertised model ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub provider: String,
    pub reachable: bool,
    pub models: Vec<String>,
    pub note: String,
}

/// List models for a provider.
///
/// - Ollama: `/api/tags` (no key).
/// - LM Studio / OpenAI / DeepSeek / Meta / OpenRouter / Custom:
///   OpenAI-compatible `/v1/models` (key required except LM Studio).
/// - Anthropic: no list API — returns the curated list ONLY when a key is
///   configured, else errors (never report curated names as live).
pub fn list_models(config: &CraftAiConfig, kind: ProviderKind) -> Result<Vec<String>, AiError> {
    if matches!(kind, ProviderKind::Anthropic) {
        let key = config.get_api_key(kind);
        if key.as_deref().unwrap_or("").trim().is_empty() {
            return Err(AiError::Config(
                "ANTHROPIC_API_KEY is not set; curated names are not a live list".to_string(),
            ));
        }
        return Ok(curated_models(kind));
    }
    if provider_needs_key(kind) {
        let key = config.get_api_key(kind);
        if key.as_deref().unwrap_or("").trim().is_empty() {
            return Err(AiError::Config(format!(
                "no API key configured for {}",
                provider_id(kind)
            )));
        }
    }
    let base = default_endpoint(config, kind);
    if matches!(kind, ProviderKind::Ollama) {
        let text = curl_text(&[format!("{base}/api/tags").as_str()], "10")?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AiError::Api(format!("unexpected /api/tags reply: {e}")))?;
        let models = value
            .get("models")
            .and_then(|m| m.as_array())
            .ok_or_else(|| AiError::Api("unexpected /api/tags reply: no models".to_string()))?;
        let mut names: Vec<String> = models
            .iter()
            .filter_map(|m| m.get("name")?.as_str().map(str::to_string))
            .collect();
        names.sort();
        return Ok(names);
    }
    // OpenAI-compatible /v1/models (LM Studio, clouds, custom).
    let url = format!("{}/v1/models", base.trim_end_matches('/'));
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sS").arg("--max-time").arg("15").arg(&url);
    if let Some(key) = config.get_api_key(kind) {
        if !key.trim().is_empty() {
            cmd.arg("-H").arg(format!("Authorization: Bearer {key}"));
        }
    }
    let output = cmd
        .output()
        .map_err(|e| AiError::Process(format!("cannot run curl.exe: {e}")))?;
    if !output.status.success() {
        return Err(AiError::Process(format!(
            "model list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| AiError::Api(format!("unexpected /v1/models reply: {e}")))?;
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| AiError::Api("unexpected /v1/models reply: no data".to_string()))?;
    let mut names: Vec<String> = data
        .iter()
        .filter_map(|m| m.get("id")?.as_str().map(str::to_string))
        .collect();
    names.sort();
    Ok(names)
}

/// Probe one provider: reachable + models (empty when unreachable).
#[must_use]
pub fn probe(config: &CraftAiConfig, kind: ProviderKind) -> ProbeReport {
    match list_models(config, kind) {
        Ok(models) => ProbeReport {
            provider: provider_id(kind).to_string(),
            reachable: true,
            models,
            note: if matches!(kind, ProviderKind::Anthropic) {
                "curated list (no list API); key is configured".to_string()
            } else {
                String::new()
            },
        },
        Err(e) => ProbeReport {
            provider: provider_id(kind).to_string(),
            reachable: false,
            models: vec![],
            note: e.to_string(),
        },
    }
}

/// VRAM switch report: which residents were evicted, whether the pick warmed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchReport {
    pub engine: bool,
    pub evicted: Vec<String>,
    pub warmed: bool,
}
/// Make `keep` the only loaded Ollama model: evict the rest, warm `keep`
/// with a 1-token generate. Empty `keep` evicts everything and warms nothing.
/// Saving a selection must switch VRAM, never just config.
pub fn switch_ollama_model(config: &CraftAiConfig, keep: &str) -> SwitchReport {
    let mut report = SwitchReport {
        engine: false,
        evicted: vec![],
        warmed: false,
    };
    let base = default_endpoint(config, ProviderKind::Ollama);
    let keep = keep.trim();
    let ps = match curl_text(&[format!("{base}/api/ps").as_str()], "10") {
        Ok(t) => t,
        Err(_) => return report,
    };
    report.engine = true;
    let value: serde_json::Value = match serde_json::from_str(&ps) {
        Ok(v) => v,
        Err(_) => return report,
    };
    let loaded: Vec<String> = value
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name")?.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if !keep.is_empty() && loaded.iter().any(|n| n == keep) {
        report.warmed = true;
    }
    for name in &loaded {
        if !keep.is_empty() && name == keep {
            continue;
        }
        let body = serde_json::json!({"model": name, "keep_alive": 0}).to_string();
        if curl_post_json(format!("{base}/api/generate").as_str(), &[], &body).is_ok() {
            report.evicted.push(name.clone());
        }
    }
    if !keep.is_empty() && !report.warmed {
        let body = serde_json::json!({
            "model": keep,
            "prompt": " ",
            "keep_alive": "10m",
            "options": {"num_predict": 1},
        })
        .to_string();
        if curl_post_json(format!("{base}/api/generate").as_str(), &[], &body).is_ok() {
            report.warmed = true;
        }
    }
    report
}

/// One Ollama resident for telemetry: name + VRAM megabytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidentInfo {
    pub name: String,
    pub size_vram_mb: u64,
}

/// Ollama residents (`/api/ps`): name + VRAM each. Empty when the engine
/// is down — callers show "provider offline", never a crash.
#[must_use]
pub fn ollama_residents(config: &CraftAiConfig) -> Vec<ResidentInfo> {
    let base = default_endpoint(config, ProviderKind::Ollama);
    let text = match curl_text(&[format!("{base}/api/ps").as_str()], "10") {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    value
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m.get("name")?.as_str()?.to_string();
                    let bytes = m
                        .get("size_vram")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    Some(ResidentInfo {
                        name,
                        size_vram_mb: bytes / 1048576,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// GPU VRAM via nvidia-smi: (used_mib, total_mib) for the first GPU.
/// None when nvidia-smi is absent (weak PC, Tier B) — not an error.
#[must_use]
pub fn gpu_vram() -> Option<(u64, u64)> {
    let output = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=memory.used,memory.total")
        .arg("--format=csv,noheader,nounits")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let mut parts = line.split(',');
    let used: u64 = parts.next()?.trim().parse().ok()?;
    let total: u64 = parts.next()?.trim().parse().ok()?;
    Some((used, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CraftAiConfig {
        CraftAiConfig::default()
    }

    #[test]
    fn provider_ids_resolve_roundtrip() {
        for id in PROVIDER_IDS {
            assert_eq!(resolve_provider(id).map(provider_id), Some(*id));
        }
    }

    #[test]
    fn legacy_aliases_resolve() {
        assert_eq!(resolve_provider("local"), Some(ProviderKind::Ollama));
        assert_eq!(resolve_provider("lms"), Some(ProviderKind::LMStudio));
        assert_eq!(resolve_provider("cloud"), Some(ProviderKind::OpenAi));
        assert_eq!(resolve_provider("banana"), None);
    }

    #[test]
    fn endpoints_have_sane_defaults() {
        let c = cfg();
        assert!(default_endpoint(&c, ProviderKind::Ollama).contains("11434"));
        assert!(default_endpoint(&c, ProviderKind::LMStudio).contains("1234"));
        assert!(default_endpoint(&c, ProviderKind::OpenAi).contains("api.openai.com"));
        assert!(default_endpoint(&c, ProviderKind::Anthropic).contains("api.anthropic.com"));
        assert!(default_endpoint(&c, ProviderKind::DeepSeek).contains("api.deepseek.com"));
        assert!(default_endpoint(&c, ProviderKind::Meta).contains("api.llama.com"));
        assert!(default_endpoint(&c, ProviderKind::OpenRouter).contains("openrouter.ai"));
    }

    #[test]
    fn only_local_providers_skip_keys() {
        assert!(!provider_needs_key(ProviderKind::Ollama));
        assert!(!provider_needs_key(ProviderKind::LMStudio));
        for id in [
            "openai",
            "anthropic",
            "deepseek",
            "meta",
            "openrouter",
            "custom",
        ] {
            assert_eq!(
                resolve_provider(id).map(provider_needs_key),
                Some(true),
                "{id} should need a key"
            );
        }
    }

    #[test]
    fn anthropic_list_without_key_is_an_error_not_a_lie() {
        let c = cfg();
        assert!(list_models(&c, ProviderKind::Anthropic).is_err());
        assert!(!curated_models(ProviderKind::Anthropic).is_empty());
    }

    #[test]
    fn cloud_list_without_key_is_an_error() {
        let c = cfg();
        assert!(list_models(&c, ProviderKind::OpenAi).is_err());
        assert!(list_models(&c, ProviderKind::DeepSeek).is_err());
    }
}
