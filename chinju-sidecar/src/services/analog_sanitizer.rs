//! Analog Sanitizer (L4 Critical)
//!
//! Converts text to image and back via OCR to physically destroy
//! steganographic information and hidden digital payloads.

use image::{Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct AnalogSanitizer;

impl AnalogSanitizer {
    /// Sanitize text by converting to image and back via OCR
    pub async fn sanitize(text: &str) -> Result<String, String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        let id = Uuid::new_v4();
        let img_path = format!("/tmp/chinju_sanitize_{}.png", id);
        
        debug!(id = %id, text_len = text.len(), "Starting analog sanitization");

        // 1. Render text to image
        let img = Self::text_to_image(text)?;
        img.save(&img_path).map_err(|e| format!("Failed to save image: {}", e))?;

        // 2. Run OCR (Tesseract)
        // Using stdout to avoid another temp file
        let output = Command::new("tesseract")
            .arg(&img_path)
            .arg("stdout")
            .arg("-l")
            .arg("eng+jpn") // Support English and Japanese
            .arg("--psm")
            .arg("6") // Assume a single uniform block of text
            .output()
            .map_err(|e| format!("Failed to execute tesseract: {}", e))?;

        // 3. Cleanup
        if let Err(e) = std::fs::remove_file(&img_path) {
            warn!("Failed to remove temp file {}: {}", img_path, e);
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("OCR failed: {}", stderr));
        }

        let result = String::from_utf8_lossy(&output.stdout).to_string();
        
        info!(
            original_len = text.len(),
            sanitized_len = result.len(),
            "Analog sanitization complete"
        );

        Ok(result.trim().to_string())
    }

    fn text_to_image(text: &str) -> Result<RgbImage, String> {
        // Estimate image size
        let lines: Vec<&str> = text.lines().collect();
        let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let line_count = lines.len();

        // Font settings
        let font_size = 20.0;
        let line_height = 24.0;
        let char_width = 12.0; // Approximation for monospace/wide chars

        let width = ((max_line_len as f32 * char_width) as u32).max(100) + 40;
        let height = ((line_count as f32 * line_height) as u32).max(100) + 40;

        // Create white canvas
        let mut image = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));

        // Load font (using a bundled font or system font)
        // For this implementation, we'll look for common system fonts
        let font_path = if std::path::Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").exists() {
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        } else if std::path::Path::new("/System/Library/Fonts/Helvetica.ttc").exists() {
            "/System/Library/Fonts/Helvetica.ttc" // macOS
        } else {
            // Fallback or error - in production, bundle the font
            return Err("No suitable font found for rendering".to_string());
        };

        let font_data = std::fs::read(font_path)
            .map_err(|e| format!("Failed to read font: {}", e))?;
        let font = Font::try_from_vec(font_data)
            .ok_or("Error constructing font")?;

        let scale = Scale { x: font_size, y: font_size };
        let color = Rgb([0, 0, 0]);

        // Draw text
        for (i, line) in lines.iter().enumerate() {
            draw_text_mut(
                &mut image,
                color,
                20,
                20 + (i as i32 * line_height as i32),
                scale,
                &font,
                line
            );
        }

        // Add some random noise to defeat adversarial examples against OCR
        // (Simple implementation: random pixel flipping)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..(width * height / 100) { // 1% noise
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            // Flip pixel slightly
            let pixel = image.get_pixel_mut(x, y);
            pixel[0] = pixel[0].saturating_add(10);
            pixel[1] = pixel[1].saturating_add(10);
            pixel[2] = pixel[2].saturating_add(10);
        }

        Ok(image)
    }
}
