//! High-level image operations
//!
//! Provides convenient functions for common image generation tasks.

use std::path::{Path, PathBuf};

use tracing::{debug, info};

use crate::error::{Error, Result};

use super::client::ImageClient;
use super::models::{ImageModel, fallback_model_ids};
use super::types::{
    ImageFormat, ImageRequest, ImageResponse, InpaintRequest, TransformRequest, UpscaleRequest,
};

/// Generate an image from a text prompt
///
/// This is the main entry point for text-to-image generation.
/// It handles model fallback and saves the result to a file.
pub async fn generate_image(
    client: &ImageClient,
    request: &ImageRequest,
    output_path: Option<&Path>,
) -> Result<ImageResponse> {
    info!(prompt = %request.prompt, model = %request.model, "Generating image");

    // Try primary model first
    let response = match client.generate(request).await {
        Ok(r) => r,
        Err(e) if should_try_fallback(&e) => {
            debug!(error = %e, "Primary model failed, trying fallbacks");
            try_fallback_models(client, request).await?
        }
        Err(e) => return Err(e),
    };

    info!(
        model = %response.model_used,
        time_ms = response.generation_time_ms,
        size_bytes = response.size_bytes(),
        "Image generated successfully"
    );

    // Save to file if output path provided
    if let Some(path) = output_path {
        save_image(&response, path)?;
        info!(path = %path.display(), "Image saved");
    }

    Ok(response)
}

/// Transform an existing image with a text prompt
pub async fn transform_image(
    client: &ImageClient,
    request: &TransformRequest,
    output_path: Option<&Path>,
) -> Result<ImageResponse> {
    info!(
        input = %request.input_image.display(),
        prompt = %request.prompt,
        model = %request.model,
        strength = request.strength,
        "Transforming image"
    );

    // Read input image
    let input_data = std::fs::read(&request.input_image)
        .map_err(|e| Error::ImageReadError(format!("{}: {}", request.input_image.display(), e)))?;

    // Check if model supports image-to-image
    if let Some(model) = ImageModel::by_id(&request.model) {
        if !model.supports_image_to_image() {
            return Err(Error::ImageModelNotAvailable(format!(
                "Model '{}' does not support image-to-image transformation",
                request.model
            )));
        }
    }

    let response = client
        .transform(&request.model, &request.prompt, &input_data, request.strength)
        .await?;

    info!(
        model = %response.model_used,
        time_ms = response.generation_time_ms,
        "Image transformed successfully"
    );

    // Save to file if output path provided
    if let Some(path) = output_path {
        save_image(&response, path)?;
        info!(path = %path.display(), "Transformed image saved");
    }

    Ok(response)
}

/// Upscale an image to higher resolution
///
/// Attempts to use a model-based upscaler if available, falls back to
/// local interpolation if no model supports upscaling.
pub async fn upscale_image(
    client: &ImageClient,
    request: &UpscaleRequest,
    output_path: Option<&Path>,
) -> Result<ImageResponse> {
    info!(
        input = %request.input_image.display(),
        scale = request.scale,
        "Upscaling image"
    );

    // Read input image
    let input_data = std::fs::read(&request.input_image)
        .map_err(|e| Error::ImageReadError(format!("{}: {}", request.input_image.display(), e)))?;

    // Try model-based upscaling first
    if let Some(model_id) = &request.model {
        if let Some(model) = ImageModel::by_id(model_id) {
            if model.supports_upscaling() {
                let prompt = format!(
                    "Upscale this image by {}x, maintaining all details and enhancing quality",
                    request.scale
                );
                let response = client.transform(model_id, &prompt, &input_data, 0.1).await?;

                if let Some(path) = output_path {
                    save_image(&response, path)?;
                }

                return Ok(response);
            }
        }
    }

    // Fallback to local upscaling using the image crate
    info!("Using local interpolation for upscaling");
    let response = local_upscale(&input_data, request.scale)?;

    if let Some(path) = output_path {
        save_image(&response, path)?;
        info!(path = %path.display(), "Upscaled image saved");
    }

    Ok(response)
}

/// Inpaint (edit) a region of an image
pub async fn inpaint_image(
    client: &ImageClient,
    request: &InpaintRequest,
    output_path: Option<&Path>,
) -> Result<ImageResponse> {
    info!(
        input = %request.input_image.display(),
        mask = %request.mask_image.display(),
        prompt = %request.prompt,
        model = %request.model,
        "Inpainting image"
    );

    // Read input images
    let input_data = std::fs::read(&request.input_image)
        .map_err(|e| Error::ImageReadError(format!("{}: {}", request.input_image.display(), e)))?;

    let mask_data = std::fs::read(&request.mask_image)
        .map_err(|e| Error::ImageReadError(format!("{}: {}", request.mask_image.display(), e)))?;

    // Check if model supports inpainting
    if let Some(model) = ImageModel::by_id(&request.model) {
        if !model.supports_inpainting() {
            return Err(Error::ImageModelNotAvailable(format!(
                "Model '{}' does not support inpainting",
                request.model
            )));
        }
    }

    let response = client
        .inpaint(&request.model, &request.prompt, &input_data, &mask_data)
        .await?;

    info!(
        model = %response.model_used,
        time_ms = response.generation_time_ms,
        "Image inpainting completed"
    );

    // Save to file if output path provided
    if let Some(path) = output_path {
        save_image(&response, path)?;
        info!(path = %path.display(), "Inpainted image saved");
    }

    Ok(response)
}

/// Try fallback models when primary fails
async fn try_fallback_models(
    client: &ImageClient,
    original_request: &ImageRequest,
) -> Result<ImageResponse> {
    let fallbacks = fallback_model_ids();
    let mut last_error = None;

    for model_id in fallbacks {
        // Skip if it's the same as the primary model
        if *model_id == original_request.model {
            continue;
        }

        debug!(model = %model_id, "Trying fallback model");

        let request = ImageRequest {
            model: model_id.to_string(),
            ..original_request.clone()
        };

        match client.generate(&request).await {
            Ok(response) => {
                info!(model = %model_id, "Fallback model succeeded");
                return Ok(response);
            }
            Err(e) => {
                debug!(model = %model_id, error = %e, "Fallback model failed");
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        Error::ImageGenerationError("All models failed".to_string())
    }))
}

/// Check if we should try fallback models for this error
fn should_try_fallback(error: &Error) -> bool {
    match error {
        Error::ImageModelNotAvailable(_) => true,
        Error::ImageGenerationError(msg) => {
            msg.contains("Rate limited") || msg.contains("overloaded") || msg.contains("unavailable")
        }
        _ => false,
    }
}

/// Save image response to file
fn save_image(response: &ImageResponse, path: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::ImageSaveError(format!("Failed to create directory: {}", e)))?;
    }

    response.save_to_file(path)
        .map_err(|e| Error::ImageSaveError(format!("{}: {}", path.display(), e)))
}

/// Local upscaling using basic interpolation
///
/// This is a fallback when no model-based upscaling is available.
fn local_upscale(input_data: &[u8], scale: u32) -> Result<ImageResponse> {
    use std::time::Instant;
    use std::io::Cursor;

    let start = Instant::now();

    // Load image
    let img = image::load_from_memory(input_data)
        .map_err(|e| Error::ImageReadError(format!("Failed to decode image: {}", e)))?;

    let (width, height) = (img.width(), img.height());
    let new_width = width * scale;
    let new_height = height * scale;

    // Resize using Lanczos3 filter for quality
    let resized = img.resize_exact(
        new_width,
        new_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Encode to PNG using write_to
    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| Error::ImageSaveError(format!("Failed to encode image: {}", e)))?;

    let generation_time = start.elapsed().as_millis() as u64;

    Ok(ImageResponse::new(
        output.into_inner(),
        ImageFormat::Png,
        "local/lanczos3".to_string(),
        generation_time,
    ))
}

/// Generate default output path for an image
pub fn generate_output_path(base_dir: &Path, format: ImageFormat) -> PathBuf {
    use chrono::Local;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("image_{}.{}", timestamp, format.extension());

    base_dir.join(filename)
}

/// Ensure API key is available
pub fn check_api_key_available() -> Result<String> {
    std::env::var("DEMIARCH_API_KEY")
        .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
        .map_err(|_| Error::ImageApiKeyMissing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_output_path() {
        let temp_dir = TempDir::new().unwrap();
        let path = generate_output_path(temp_dir.path(), ImageFormat::Png);

        assert!(path.to_string_lossy().ends_with(".png"));
        assert!(path.starts_with(temp_dir.path()));
    }

    #[test]
    fn test_should_try_fallback() {
        let model_error = Error::ImageModelNotAvailable("test".to_string());
        assert!(should_try_fallback(&model_error));

        let rate_limit = Error::ImageGenerationError("Rate limited".to_string());
        assert!(should_try_fallback(&rate_limit));

        let other_error = Error::ImageReadError("test".to_string());
        assert!(!should_try_fallback(&other_error));
    }
}
