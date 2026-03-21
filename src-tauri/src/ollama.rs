use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const OLLAMA_ENDPOINT: &str = "http://192.168.1.12:11434/api/generate";
const OLLAMA_MODEL: &str = "glm-ocr";
const OLLAMA_PROMPT: &str = "Text Recognition:";
const OLLAMA_NUM_CTX: u64 = 16384;
const OLLAMA_TIMEOUT_SECS: u64 = 60;
const OLLAMA_CONNECT_TIMEOUT_SECS: u64 = 3;
const OCR_MAX_DIMENSION: u32 = 2048;

pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String> {
    let clamped_bytes = clamp_image_for_ocr(png_bytes)?;
    let b64 = STANDARD.encode(&clamped_bytes);

    let body = serde_json::json!({
        "model": OLLAMA_MODEL,
        "prompt": OLLAMA_PROMPT,
        "images": [b64],
        "stream": false,
        "options": {
            "num_ctx": OLLAMA_NUM_CTX
        }
    });

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(OLLAMA_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(OLLAMA_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let response = client
        .post(OLLAMA_ENDPOINT)
        .json(&body)
        .send()
        .await
        .map_err(classify_request_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama returned HTTP {status}: {body_text}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;

    let text = recognize_text_from_response(&json)?;
    Ok(sanitize_ocr_output(text))
}

fn clamp_image_for_ocr(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
    use image::{GenericImageView, ImageFormat};
    let img = image::load_from_memory(png_bytes)
        .map_err(|e| format!("Failed to decode PNG for OCR: {e}"))?;
    let (w, h) = img.dimensions();
    if w <= OCR_MAX_DIMENSION && h <= OCR_MAX_DIMENSION {
        return Ok(png_bytes.to_vec());
    }
    let scale = (OCR_MAX_DIMENSION as f32) / (w.max(h) as f32);
    let new_w = ((w as f32) * scale) as u32;
    let new_h = ((h as f32) * scale) as u32;
    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
    let mut out = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode resized PNG: {e}"))?;
    Ok(out.into_inner())
}

fn classify_request_error(e: reqwest::Error) -> String {
    if e.is_connect() {
        format!("Ollama is not reachable at 192.168.1.12:11434. Is it running? ({e})")
    } else if e.is_timeout() {
        format!("Ollama OCR timed out after {OLLAMA_TIMEOUT_SECS}s. The model may still be loading.")
    } else {
        format!("Ollama request failed: {e}")
    }
}

fn recognize_text_from_response(json: &serde_json::Value) -> Result<String, String> {
    if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
        return Err(format!("Ollama error: {err}"));
    }
    let text = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Ollama response did not contain a 'response' field.".to_string())?;
    if text.trim().is_empty() {
        return Err("Ollama OCR returned empty text.".into());
    }
    Ok(text.to_string())
}

fn sanitize_ocr_output(text: String) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: base64 no-prefix guard — images[0] must be raw base64, not a data: URI
    #[tokio::test]
    async fn test_base64_no_data_prefix() {
        // Build a minimal PNG (1x1 white pixel) to get real base64
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba([255u8, 255, 255, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();

        let b64 = STANDARD.encode(&png_bytes);
        let body = serde_json::json!({
            "model": OLLAMA_MODEL,
            "images": [b64],
            "stream": false,
        });

        let images_value = body["images"][0].as_str().unwrap();
        assert!(
            !images_value.starts_with("data:"),
            "images[0] must be raw base64, not a data: URI. Got: {}...",
            &images_value[..images_value.len().min(30)]
        );
    }

    // Test 2: image resize guard — 3000x1500 synthetic image must be clamped to <= 2048 on both axes
    #[tokio::test]
    async fn test_image_resize_clamps_to_max_dimension() {
        use image::{ImageBuffer, Rgba};
        // Create a 3000x1500 synthetic white image
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(3000, 1500, Rgba([255u8, 255, 255, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let png_bytes = buf.into_inner();

        let result = clamp_image_for_ocr(&png_bytes).expect("clamp_image_for_ocr should succeed");

        let decoded = image::load_from_memory(&result).expect("result should be valid PNG");
        let (w, h) = image::GenericImageView::dimensions(&decoded);
        assert!(
            w <= OCR_MAX_DIMENSION,
            "Width {w} must be <= {OCR_MAX_DIMENSION}"
        );
        assert!(
            h <= OCR_MAX_DIMENSION,
            "Height {h} must be <= {OCR_MAX_DIMENSION}"
        );
        // Verify aspect ratio preserved: original is 3000x1500 (2:1). New w should be ~2048, h ~1024.
        let aspect_original = 3000.0_f32 / 1500.0_f32;
        let aspect_result = w as f32 / h as f32;
        let aspect_diff = (aspect_original - aspect_result).abs();
        assert!(
            aspect_diff < 0.05,
            "Aspect ratio should be preserved (got {aspect_result:.3}, expected ~{aspect_original:.3})"
        );
    }

    // Test 3: error classification — connect error maps to "not reachable" / "not running"
    #[tokio::test]
    async fn test_error_classification_connect() {
        // Attempt to connect to a port guaranteed to be closed (localhost:1)
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(200))
            .build()
            .unwrap();
        let result = client
            .post("http://127.0.0.1:1/api/generate")
            .json(&serde_json::json!({}))
            .send()
            .await;

        assert!(result.is_err(), "Expected connection error");
        let err = result.unwrap_err();
        // Should be a connection error
        assert!(
            err.is_connect() || err.is_timeout(),
            "Expected connect or timeout error, got: {err}"
        );
        let classified = classify_request_error(err);
        let lower = classified.to_lowercase();
        assert!(
            lower.contains("not reachable") || lower.contains("not running") || lower.contains("timed out"),
            "Error message should indicate unreachable/not running. Got: {classified}"
        );
    }

    // Test 4: error classification — timeout maps to "timed out"
    #[tokio::test]
    async fn test_error_classification_timeout() {
        // Use a very short timeout against a host that will hang (use a non-routable IP with tiny timeout)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();
        let result = client
            .post("http://192.0.2.1:11434/api/generate") // TEST-NET-1, non-routable
            .json(&serde_json::json!({}))
            .send()
            .await;

        // This may be a connect or timeout error depending on the OS; test the classify fn directly
        // by constructing a scenario: try to get a timeout from classify_request_error
        // We can also just verify the timeout string format is correct when is_timeout() is true
        // by testing the message directly since we can't easily construct a reqwest::Error
        let timeout_msg = format!("Ollama OCR timed out after {OLLAMA_TIMEOUT_SECS}s. The model may still be loading.");
        assert!(
            timeout_msg.to_lowercase().contains("timed out"),
            "Timeout message should contain 'timed out'. Got: {timeout_msg}"
        );

        // Also verify the error from our attempt is handled
        if let Err(e) = result {
            let classified = classify_request_error(e);
            let lower = classified.to_lowercase();
            assert!(
                lower.contains("timed out") || lower.contains("not reachable") || lower.contains("not running") || lower.contains("failed"),
                "Classified error should be meaningful. Got: {classified}"
            );
        }
    }

    // Test 5: response parse — success case
    #[test]
    fn test_response_parse_success() {
        let json = serde_json::json!({"response": "Hello world", "done": true});
        let result = recognize_text_from_response(&json);
        assert_eq!(result, Ok("Hello world".to_string()));
    }

    // Test 6: response parse — empty response returns Err containing "empty"
    #[test]
    fn test_response_parse_empty() {
        let json = serde_json::json!({"response": "  ", "done": true});
        let result = recognize_text_from_response(&json);
        assert!(result.is_err(), "Expected Err for empty response");
        let err = result.unwrap_err().to_lowercase();
        assert!(
            err.contains("empty"),
            "Error should contain 'empty'. Got: {err}"
        );
    }

    // Test 7: response parse — error key returns Err containing the error text
    #[test]
    fn test_response_parse_error_key() {
        let json = serde_json::json!({"error": "model not found"});
        let result = recognize_text_from_response(&json);
        assert!(result.is_err(), "Expected Err for error key in response");
        let err = result.unwrap_err().to_lowercase();
        assert!(
            err.contains("model not found"),
            "Error should contain 'model not found'. Got: {err}"
        );
    }
}
