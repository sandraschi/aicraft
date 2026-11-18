//! DeepSeek app API, split the way the service actually splits it.
//!
//! Chat completions (`deepseek-flash`) are one call. Search and URL reading are
//! tools on that call. The service then ignores temperature.
//!
//! OCR, speech-to-text, and text-to-speech are not chat tools. Each is its own
//! HTTP endpoint. OCR returns page text and stops. It does not ask the model a
//! question. Speech-to-text accepts one short audio file and returns a transcript.
//! Text-to-speech accepts text and returns a wav.

use crate::{AiError, CraftAiConfig, ProviderKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const FLASH_MODEL: &str = "deepseek-flash";
pub const CHAT_URL: &str = "https://api.deepseek.com/chat/completions";
pub const OCR_URL: &str = "https://api.deepseek.com/v1/tools/ocr";
pub const ASR_URL: &str = "https://api.deepseek.com/v1/tools/asr";
pub const TTS_URL: &str = "https://api.deepseek.com/v1/tools/tts";

/// Off-peak cache-miss input, USD per million tokens. Peak hours are double.
pub const FLASH_INPUT_USD_PER_M: f64 = 0.15;
/// Off-peak output, USD per million tokens.
pub const FLASH_OUTPUT_USD_PER_M: f64 = 0.60;
/// One web search. The published price is $0.75 per 1,000 searches.
pub const SEARCH_USD: f64 = 0.00075;
/// One page fetch. Same published rate as search.
pub const FETCH_USD: f64 = 0.00075;
/// One parsed or OCR page. The published price is $0.225 per 1,000 pages.
pub const PAGE_USD: f64 = 0.000225;

#[derive(Debug, Clone, PartialEq)]
pub struct DeepSeekReply {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub search_requests: u64,
    pub fetch_requests: u64,
}

impl DeepSeekReply {
    #[must_use]
    pub fn estimate_usd(&self) -> f64 {
        tool_fee_usd(self.search_requests, self.fetch_requests, 0)
            + (self.input_tokens as f64 * FLASH_INPUT_USD_PER_M
                + self.output_tokens as f64 * FLASH_OUTPUT_USD_PER_M)
                / 1_000_000.0
    }
}

#[must_use]
pub fn tool_fee_usd(searches: u64, fetches: u64, pages: u64) -> f64 {
    searches as f64 * SEARCH_USD + fetches as f64 * FETCH_USD + pages as f64 * PAGE_USD
}

/// Key saved in `~/.craft/ai_config.json`, or `DEEPSEEK_API_KEY`.
pub fn api_key() -> Result<String, AiError> {
    CraftAiConfig::load()
        .get_api_key(ProviderKind::DeepSeek)
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            AiError::Config(
                "DeepSeek API key is not set. Save it in CoderCraft LLM settings, or set DEEPSEEK_API_KEY."
                    .to_string(),
            )
        })
}

/// Use an explicit deepseek model id. Anything else becomes `deepseek-flash`,
/// so a leftover local model name is not sent to the cloud.
#[must_use]
pub fn resolve_model(requested: &str) -> String {
    let trimmed = requested.trim();
    if trimmed.to_ascii_lowercase().contains("deepseek") {
        trimmed.to_string()
    } else {
        FLASH_MODEL.to_string()
    }
}

#[must_use]
pub fn completion_body(model: &str, system: &str, user: &str, research: bool) -> Value {
    let mut messages = Vec::new();
    if !system.trim().is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    messages.push(json!({"role": "user", "content": user}));
    if research {
        json!({
            "model": model,
            "thinking": {"type": "enabled"},
            "tools": [
                {"type": "deepseek_web_search"},
                {"type": "deepseek_read_url"}
            ],
            "messages": messages,
            "stream": false
        })
    } else {
        json!({
            "model": model,
            "temperature": 0.2,
            "messages": messages,
            "stream": false
        })
    }
}

/// Plain `deepseek-flash` completion. Temperature stays at 0.2 because search,
/// URL reading, the safety check, and titles are left off.
pub fn complete_flash(model: &str, system: &str, user: &str) -> Result<String, AiError> {
    let key = api_key()?;
    let body = completion_body(&resolve_model(model), system, user, false);
    let raw = curl_json(CHAT_URL, &key, &body, "90")?;
    Ok(parse_completion(&raw)?.text)
}

/// Search plus URL reading on `deepseek-flash`. Temperature is omitted because
/// DeepSeek ignores it once those tools are on.
pub fn research(system: &str, user: &str) -> Result<DeepSeekReply, AiError> {
    let key = api_key()?;
    let body = completion_body(FLASH_MODEL, system, user, true);
    let raw = curl_json(CHAT_URL, &key, &body, "120")?;
    parse_completion(&raw)
}

/// Recognize one document or image. The file is not stored and is not given to
/// the chat model. `text_recognition` extracts Word and PowerPoint text directly
/// when the file has no images.
pub fn ocr_file(path: &Path) -> Result<String, AiError> {
    let key = api_key()?;
    if !path.is_file() {
        return Err(AiError::Config(format!(
            "OCR file does not exist: {}",
            path.display()
        )));
    }
    let raw = curl_file(
        OCR_URL,
        &key,
        "180",
        &[("purpose", "text_recognition")],
        "file",
        path,
    )?;
    let pages = parse_ocr(&raw)?;
    if pages.is_empty() {
        return Err(AiError::EmptyResponse);
    }
    let mut out = String::new();
    for page in pages {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("--- page {} ---\n", page.index + 1));
        out.push_str(page.text.trim());
        if page.finish_reason == "length" {
            out.push_str("\n[page truncated]");
        }
    }
    Ok(out)
}

/// Transcribe one WAV or Ogg Opus file, at most 120 seconds and 8 MiB.
pub fn transcribe_file(path: &Path) -> Result<String, AiError> {
    let key = api_key()?;
    if !path.is_file() {
        return Err(AiError::Config(format!(
            "audio file does not exist: {}",
            path.display()
        )));
    }
    let raw = curl_file(ASR_URL, &key, "60", &[], "file", path)?;
    parse_asr(&raw)
}

/// Write a wav for `text`. Voices are `stella` (default) and `tide`.
/// At most 10,000 characters.
pub fn speak_to_file(text: &str, dest: &Path) -> Result<PathBuf, AiError> {
    let key = api_key()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AiError::Config("nothing to read aloud".to_string()));
    }
    let chars = trimmed.chars().count();
    if chars > 10_000 {
        return Err(AiError::Config(format!(
            "text-to-speech accepts at most 10,000 characters ({chars} given)"
        )));
    }
    let body = json!({
        "text": trimmed,
        "voice_id": "stella",
        "format": "wav",
        "filter_markdown": true
    });
    curl_audio(TTS_URL, &key, &body, dest, "120")?;
    Ok(dest.to_path_buf())
}

pub fn parse_completion(body: &str) -> Result<DeepSeekReply, AiError> {
    let value: Value = serde_json::from_str(body.trim())
        .map_err(|e| AiError::Api(format!("DeepSeek reply is not JSON: {e}")))?;
    if let Some(err) = value.get("error") {
        return Err(AiError::Api(format!("DeepSeek error: {err}")));
    }
    let message = value
        .pointer("/choices/0/message")
        .ok_or_else(|| AiError::Api(format!("DeepSeek reply has no message: {body}")))?;
    let text = message_text(message);
    if text.trim().is_empty() {
        return Err(AiError::EmptyResponse);
    }
    let usage = value.get("usage").cloned().unwrap_or(Value::Null);
    let tools = usage.get("server_tool_use").cloned().unwrap_or(Value::Null);
    Ok(DeepSeekReply {
        text,
        input_tokens: num_field(&usage, &["prompt_tokens", "input_tokens"]),
        output_tokens: num_field(&usage, &["completion_tokens", "output_tokens"]),
        search_requests: num_field(&tools, &["web_search_requests"]),
        fetch_requests: num_field(&tools, &["web_fetch_requests"]),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrPage {
    pub index: u64,
    pub text: String,
    pub finish_reason: String,
}

pub fn parse_ocr(body: &str) -> Result<Vec<OcrPage>, AiError> {
    let value: Value = serde_json::from_str(body.trim())
        .map_err(|e| AiError::Api(format!("OCR reply is not JSON: {e}")))?;
    if let Some(err) = value.get("error") {
        return Err(AiError::Api(format!("OCR error: {err}")));
    }
    let pages = value
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| AiError::Api("OCR reply has no pages".to_string()))?;
    let mut out = Vec::new();
    for page in pages {
        out.push(OcrPage {
            index: num_field(page, &["index"]),
            text: page
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            finish_reason: page
                .get("finish_reason")
                .and_then(|t| t.as_str())
                .unwrap_or("stop")
                .to_string(),
        });
    }
    Ok(out)
}

pub fn parse_asr(body: &str) -> Result<String, AiError> {
    let value: Value = serde_json::from_str(body.trim())
        .map_err(|e| AiError::Api(format!("speech-to-text reply is not JSON: {e}")))?;
    if let Some(err) = value.get("error") {
        return Err(AiError::Api(format!("speech-to-text error: {err}")));
    }
    value
        .get("text")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .ok_or(AiError::EmptyResponse)
}

fn message_text(message: &Value) -> String {
    match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => {
            let mut text = String::new();
            for block in blocks {
                let kind = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if kind == "thinking" {
                    continue;
                }
                if let Some(piece) = block.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() && !piece.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(piece);
                }
            }
            text
        }
        _ => String::new(),
    }
}

fn num_field(value: &Value, keys: &[&str]) -> u64 {
    for key in keys {
        if let Some(n) = value.get(*key).and_then(|v| v.as_u64()) {
            return n;
        }
    }
    0
}

fn curl_json(url: &str, key: &str, body: &Value, max_time: &str) -> Result<String, AiError> {
    let body_str = serde_json::to_string(body)?;
    let mut cmd = Command::new("curl.exe");
    cmd.arg("-sS")
        .arg("--max-time")
        .arg(max_time)
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {key}"))
        .arg("--data-binary")
        .arg(body_str);
    run_curl(cmd)
}

fn curl_file(
    url: &str,
    key: &str,
    max_time: &str,
    fields: &[(&str, &str)],
    file_field: &str,
    path: &Path,
) -> Result<String, AiError> {
    let mut cmd = Command::new("curl.exe");
    cmd.arg("-sS")
        .arg("--max-time")
        .arg(max_time)
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg(format!("Authorization: Bearer {key}"));
    for (name, value) in fields {
        cmd.arg("-F").arg(format!("{name}={value}"));
    }
    cmd.arg("-F")
        .arg(format!("{file_field}=@{}", path.display()));
    run_curl(cmd)
}

fn curl_audio(
    url: &str,
    key: &str,
    body: &Value,
    dest: &Path,
    max_time: &str,
) -> Result<(), AiError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AiError::Process(e.to_string()))?;
    }
    let body_str = serde_json::to_string(body)?;
    let mut cmd = Command::new("curl.exe");
    cmd.arg("-sS")
        .arg("--max-time")
        .arg(max_time)
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {key}"))
        .arg("--data-binary")
        .arg(body_str)
        .arg("--output")
        .arg(dest);
    let output = cmd
        .output()
        .map_err(|e| AiError::Process(format!("cannot run curl.exe: {e}")))?;
    if !output.status.success() {
        return Err(AiError::Process(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let bytes = std::fs::read(dest).map_err(|e| AiError::Process(e.to_string()))?;
    if bytes.first().copied() == Some(b'{') {
        let text = String::from_utf8_lossy(&bytes);
        let _ = std::fs::remove_file(dest);
        return Err(AiError::Api(format!("text-to-speech error: {text}")));
    }
    if bytes.len() < 44 {
        return Err(AiError::EmptyResponse);
    }
    Ok(())
}

fn run_curl(mut cmd: Command) -> Result<String, AiError> {
    let output = cmd
        .output()
        .map_err(|e| AiError::Process(format!("cannot run curl.exe: {e}")))?;
    if !output.status.success() {
        return Err(AiError::Process(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        return Err(AiError::EmptyResponse);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_completion_keeps_temperature_and_omits_tools() {
        let body = completion_body("deepseek-flash", "sys", "hi", false);
        assert_eq!(body["temperature"], 0.2);
        assert!(body.get("tools").is_none());
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn research_omits_temperature_and_names_the_tools() {
        let body = completion_body(FLASH_MODEL, "sys", "hi", true);
        assert!(body.get("temperature").is_none());
        assert_eq!(body["model"], FLASH_MODEL);
        assert_eq!(body["tools"][0]["type"], "deepseek_web_search");
        assert_eq!(body["tools"][1]["type"], "deepseek_read_url");
    }

    #[test]
    fn leftover_local_model_names_become_flash() {
        assert_eq!(resolve_model("gemma4:12b"), FLASH_MODEL);
        assert_eq!(resolve_model("deepseek-chat"), "deepseek-chat");
    }

    #[test]
    fn completion_parser_skips_thinking_and_counts_tools() {
        let raw = r#"{
            "choices": [{"message": {"content": [
                {"type": "thinking", "thinking": "hidden"},
                {"type": "text", "text": "Cited answer"}
            ]}}],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 3,
                "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 2}
            }
        }"#;
        let reply = parse_completion(raw).unwrap_or(DeepSeekReply {
            text: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            search_requests: 0,
            fetch_requests: 0,
        });
        assert_eq!(reply.text, "Cited answer");
        assert_eq!(reply.search_requests, 1);
        assert_eq!(reply.fetch_requests, 2);
        assert!(reply.estimate_usd() > 0.0);
    }

    #[test]
    fn ocr_and_asr_parsers_read_their_fields() {
        let ocr = r#"{"pages":[{"index":0,"text":"Hello","finish_reason":"stop"}]}"#;
        let pages = parse_ocr(ocr).unwrap_or_default();
        assert_eq!(pages.first().map(|p| p.text.as_str()), Some("Hello"));
        let asr = r#"{"text":"What is the weather","duration_ms":3200}"#;
        assert_eq!(parse_asr(asr).unwrap_or_default(), "What is the weather");
    }

    #[test]
    fn tool_fee_for_a_thousand_searches_is_the_published_price() {
        let fee = tool_fee_usd(1000, 0, 0);
        assert!((fee - 0.75).abs() < 1e-9);
    }
}
