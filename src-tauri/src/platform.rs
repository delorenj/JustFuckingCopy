use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use image::{DynamicImage, GenericImageView, ImageFormat};

const VISION_OCR_SCRIPT: &str = include_str!("../scripts/vision_ocr.swift");

pub fn capture_snapshot() -> Result<(Vec<u8>, u32, u32), String> {
    let path = temp_path("snapshot", "png");

    #[cfg(target_os = "macos")]
    run_capture_command(
        Command::new("screencapture")
            .arg("-x")
            .arg("-t")
            .arg("png")
            .arg(&path),
        "macOS screencapture failed",
    )?;

    #[cfg(target_os = "linux")]
    capture_linux(&path)?;

    #[cfg(target_os = "windows")]
    capture_windows(&path)?;

    let bytes = fs::read(&path).map_err(|error| format!("Failed to read screenshot: {error}"))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("Failed to decode screenshot: {error}"))?;
    let (width, height) = image.dimensions();
    let _ = fs::remove_file(&path);

    Ok((bytes, width, height))
}

pub fn crop_png(bytes: &[u8], x: u32, y: u32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("Failed to decode stored snapshot: {error}"))?;
    let bounded_width = width.min(image.width().saturating_sub(x));
    let bounded_height = height.min(image.height().saturating_sub(y));

    if bounded_width == 0 || bounded_height == 0 {
        return Err("Selection fell outside the current snapshot.".into());
    }

    let cropped = image.crop_imm(x, y, bounded_width, bounded_height);
    let mut output = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| format!("Failed to encode crop: {error}"))?;
    Ok(output.into_inner())
}

pub fn recognize_text_from_png(bytes: &[u8]) -> Result<String, String> {
    let path = temp_path("ocr-crop", "png");
    fs::write(&path, bytes).map_err(|error| format!("Failed to write OCR crop: {error}"))?;

    let result = recognize_text_from_file(&path);
    let _ = fs::remove_file(&path);
    result
}

#[cfg(target_os = "macos")]
fn recognize_text_from_file(path: &Path) -> Result<String, String> {
    let mut process = Command::new("swift")
        .arg("-")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start Swift OCR bridge: {error}"))?;

    if let Some(stdin) = process.stdin.as_mut() {
        stdin
            .write_all(VISION_OCR_SCRIPT.as_bytes())
            .map_err(|error| format!("Failed to send Vision OCR script to Swift: {error}"))?;
    }

    let output = process
        .wait_with_output()
        .map_err(|error| format!("Vision OCR process failed: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Vision OCR failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(sanitize_ocr_output(
        String::from_utf8_lossy(&output.stdout).to_string(),
    ))
}

#[cfg(target_os = "linux")]
fn recognize_text_from_file(path: &Path) -> Result<String, String> {
    let output = Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .arg("--psm")
        .arg("6")
        .output()
        .map_err(|error| {
            format!("Failed to launch tesseract. Install it or wire another OCR backend. {error}")
        })?;

    if !output.status.success() {
        return Err(format!(
            "tesseract OCR failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(sanitize_ocr_output(
        String::from_utf8_lossy(&output.stdout).to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn recognize_text_from_file(_path: &Path) -> Result<String, String> {
    Err("Windows OCR backend is not implemented yet.".into())
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

fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("{prefix}-{timestamp}.{extension}"))
}

fn run_capture_command(command: &mut Command, error_prefix: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{error_prefix}: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{error_prefix}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(target_os = "linux")]
fn capture_linux(path: &Path) -> Result<(), String> {
    let mut attempts = vec![
        {
            let mut command = Command::new("grim");
            command.arg("-t").arg("png").arg(path);
            command
        },
        {
            let mut command = Command::new("gnome-screenshot");
            command.arg("-f").arg(path);
            command
        },
        {
            let mut command = Command::new("import");
            command.arg("-window").arg("root").arg(path);
            command
        },
    ];

    let mut last_error = None;
    for attempt in attempts.iter_mut() {
        match attempt.output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                last_error = Some(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    Err(format!(
        "No Linux screenshot backend succeeded. Tried grim, gnome-screenshot, and import. Last error: {}",
        last_error.unwrap_or_else(|| "unknown error".into())
    ))
}

#[cfg(target_os = "windows")]
fn capture_windows(path: &Path) -> Result<(), String> {
    let escaped_path = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen; \
         $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height; \
         $graphics = [System.Drawing.Graphics]::FromImage($bitmap); \
         $graphics.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bitmap.Size); \
         $bitmap.Save('{escaped_path}', [System.Drawing.Imaging.ImageFormat]::Png); \
         $graphics.Dispose(); \
         $bitmap.Dispose();"
    );

    run_capture_command(
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(script),
        "Windows PowerShell screen capture failed",
    )
}

#[allow(dead_code)]
pub fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("Failed to decode PNG dimensions: {error}"))?;
    Ok(image.dimensions())
}

#[allow(dead_code)]
pub fn decode_png(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|error| format!("Failed to decode PNG: {error}"))
}
