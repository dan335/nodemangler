/// Shared utilities for AI operations: HTTP helpers, API key resolution,
/// base64 image encode/decode, and cost estimation.

use crate::float_image::FloatImage;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};

/// Timeout for AI API requests (120 seconds).
const AI_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

// ─── Cost Estimation ────────────────────────────────────────────────────────

/// Per-token rates for gpt-image-1 (USD).
const GPT_IMAGE_1_INPUT_TOKEN_COST: f64 = 0.00001;
const GPT_IMAGE_1_OUTPUT_TOKEN_COST: f64 = 0.00004;

/// Estimate cost from an OpenAI image API response.
///
/// For gpt-image-1: reads the `usage` field and computes from token counts.
/// For DALL-E 2/3: uses a static per-image pricing table.
pub fn estimate_cost_from_response(
    json: &serde_json::Value,
    model: &str,
    size: &str,
    quality: &str,
) -> f64 {
    // gpt-image-1 returns token-based usage in the response.
    if let Some(usage) = json.get("usage") {
        let input_tokens = usage["input_tokens"].as_f64().unwrap_or(0.0);
        let output_tokens = usage["output_tokens"].as_f64().unwrap_or(0.0);
        return input_tokens * GPT_IMAGE_1_INPUT_TOKEN_COST
            + output_tokens * GPT_IMAGE_1_OUTPUT_TOKEN_COST;
    }

    // DALL-E 2/3: fixed per-image pricing by model, size, and quality.
    dalle_fixed_cost(model, size, quality)
}

/// Static pricing table for DALL-E 2 and DALL-E 3 (USD per image).
fn dalle_fixed_cost(model: &str, size: &str, quality: &str) -> f64 {
    match model {
        "dall-e-3" => match (size, quality) {
            ("1024x1024", "standard") => 0.040,
            ("1024x1024", "hd") => 0.080,
            ("1024x1792" | "1792x1024", "standard") => 0.080,
            ("1024x1792" | "1792x1024", "hd") => 0.120,
            _ => 0.040, // fallback to standard 1024x1024
        },
        "dall-e-2" => match size {
            "1024x1024" => 0.020,
            "512x512" => 0.018,
            "256x256" => 0.016,
            _ => 0.020,
        },
        // Unknown model — return 0 rather than guessing.
        _ => 0.0,
    }
}

// ─── Session Cost Tracking (thread-safe) ────────────────────────────────────

/// Atomic storage for session cost in USD (f64 stored as u64 bits).
static SESSION_COST_BITS: AtomicU64 = AtomicU64::new(0);
/// Atomic storage for cost limit in USD (f64 stored as u64 bits). 0 = no limit.
static COST_LIMIT_BITS: AtomicU64 = AtomicU64::new(0);

/// Get the current session cost in USD.
pub fn get_session_cost() -> f64 {
    f64::from_bits(SESSION_COST_BITS.load(Ordering::Relaxed))
}

/// Add cost to the session total. Returns the new total.
pub fn add_session_cost(cost: f64) -> f64 {
    loop {
        let old_bits = SESSION_COST_BITS.load(Ordering::Relaxed);
        let new_val = f64::from_bits(old_bits) + cost;
        if SESSION_COST_BITS
            .compare_exchange(old_bits, new_val.to_bits(), Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return new_val;
        }
    }
}

/// Reset session cost to zero (called on app startup).
pub fn reset_session_cost() {
    SESSION_COST_BITS.store(0u64, Ordering::Relaxed);
}

/// Get the current cost limit in USD. 0 means no limit.
pub fn get_cost_limit() -> f64 {
    f64::from_bits(COST_LIMIT_BITS.load(Ordering::Relaxed))
}

/// Set the cost limit in USD. 0 means no limit.
pub fn set_cost_limit(limit: f64) {
    COST_LIMIT_BITS.store(limit.to_bits(), Ordering::Relaxed);
}

/// Check whether a new AI request would exceed the session cost limit.
/// Returns `Ok(())` if within budget, or an error message if the limit would be exceeded.
pub fn check_cost_limit() -> Result<(), String> {
    let limit = get_cost_limit();
    let spent = get_session_cost();
    if limit > 0.0 && spent >= limit {
        Err(format!(
            "AI cost limit of ${:.2} reached (spent ${:.2} this session). \
             Increase the limit in Settings to continue.",
            limit, spent
        ))
    } else {
        Ok(())
    }
}

// ─── API Key Resolution ──────────────────────────────────────────────────────

/// Resolve an API key from the node input or environment variable.
///
/// Priority:
/// 1. If `key_input` is non-empty (after trimming), use it directly.
/// 2. Otherwise, check `std::env::var(env_var_name)`.
/// 3. If neither is available, return a descriptive error.
pub fn resolve_api_key(key_input: &str, env_var_name: &str) -> Result<String, String> {
    let trimmed = key_input.trim();
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }

    match std::env::var(env_var_name) {
        Ok(val) if !val.trim().is_empty() => Ok(val.trim().to_string()),
        _ => Err(format!(
            "API key required. Set the 'api key' input, the {} env var, or configure it in Settings > API Keys.",
            env_var_name
        )),
    }
}

// ─── HTTP Helpers ────────────────────────────────────────────────────────────

/// Build a reqwest client with the AI timeout configured.
fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(AI_REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// POST JSON to an API endpoint with Bearer auth, return parsed JSON response.
pub async fn make_ai_request(
    url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let client = build_client()?;

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Parse response JSON.
    let json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    // Check for API error responses.
    if !status.is_success() {
        let error_msg = json["error"]["message"]
            .as_str()
            .unwrap_or("Unknown API error");
        return Err(format!("API error ({}): {}", status.as_u16(), error_msg));
    }

    Ok(json)
}

/// POST multipart form data with Bearer auth, return parsed JSON response.
pub async fn make_ai_multipart_request(
    url: &str,
    api_key: &str,
    form: reqwest::multipart::Form,
) -> Result<serde_json::Value, String> {
    let client = build_client()?;

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    if !status.is_success() {
        let error_msg = json["error"]["message"]
            .as_str()
            .unwrap_or("Unknown API error");
        return Err(format!("API error ({}): {}", status.as_u16(), error_msg));
    }

    Ok(json)
}

// ─── Image Encoding / Decoding ───────────────────────────────────────────────

/// Convert a FloatImage to PNG bytes.
/// Converts to RGBA8 first since PNG doesn't support f32 pixel formats.
pub fn float_image_to_png_bytes(image: &FloatImage) -> Result<Vec<u8>, String> {
    let rgba8 = image.to_rgba8();
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba8)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image as PNG: {}", e))?;
    Ok(buf.into_inner())
}

/// Convert a FloatImage to a base64-encoded PNG string.
pub fn encode_float_image_to_base64_png(image: &FloatImage) -> Result<String, String> {
    use base64::Engine;
    let bytes = float_image_to_png_bytes(image)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Decode a base64 string (PNG or JPEG) into a FloatImage.
pub fn decode_base64_to_float_image(b64: &str) -> Result<FloatImage, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let dynamic = image::load_from_memory(&bytes)
        .map_err(|e| format!("Failed to decode image from bytes: {}", e))?;

    Ok(FloatImage::from_dynamic(&dynamic))
}

/// Parse an OpenAI image generation/edit response and extract the image.
///
/// Expects JSON with `data[0].b64_json` (base64-encoded image).
/// Returns the decoded FloatImage, width, height, and optional revised prompt.
pub fn parse_openai_image_response(
    json: &serde_json::Value,
) -> Result<(FloatImage, i32, i32, Option<String>), String> {
    let data = json["data"]
        .as_array()
        .ok_or_else(|| "Response missing 'data' array.".to_string())?;

    if data.is_empty() {
        return Err("Response 'data' array is empty.".to_string());
    }

    let b64 = data[0]["b64_json"]
        .as_str()
        .ok_or_else(|| "Response missing 'b64_json' field.".to_string())?;

    let image = decode_base64_to_float_image(b64)?;
    let width = image.width() as i32;
    let height = image.height() as i32;

    // DALL-E 3 may return a revised prompt.
    let revised_prompt = data[0]["revised_prompt"]
        .as_str()
        .map(|s| s.to_string());

    Ok((image, width, height, revised_prompt))
}

#[cfg(test)]
#[path = "shared_tests.rs"]
mod tests;
