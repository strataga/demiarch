//! Image generation commands
//!
//! CLI command implementations for image generation operations.

use std::path::PathBuf;

use tracing::info;

use crate::error::{Error, Result};
use crate::image::operations::{check_api_key_available, generate_output_path};
use crate::image::{
    generate_image, inpaint_image, transform_image, upscale_image, ImageClient, ImageFormat,
    ImageRequest, ImageSize, ImageStyle, InpaintRequest, TransformRequest, UpscaleRequest,
    IMAGE_MODELS,
};

/// Generate an image from a text prompt
pub async fn generate(
    prompt: String,
    output: Option<PathBuf>,
    size: Option<String>,
    style: Option<String>,
    model: Option<String>,
    negative: Option<String>,
    seed: Option<u64>,
) -> Result<PathBuf> {
    let api_key = check_api_key_available()?;

    let client = ImageClient::new(api_key)?;

    // Build request
    let mut request = ImageRequest::new(prompt);

    if let Some(m) = model {
        request = request.with_model(m);
    }

    if let Some(s) = size {
        let image_size = ImageSize::parse(&s).ok_or_else(|| {
            Error::InvalidInput(format!(
                "Invalid size '{}'. Use: square, portrait, landscape, or WxH (e.g., 1024x768)",
                s
            ))
        })?;
        request = request.with_size(image_size);
    }

    if let Some(s) = style {
        let image_style = ImageStyle::parse(&s).ok_or_else(|| {
            Error::InvalidInput(format!(
                "Invalid style '{}'. Use: vivid, natural, photorealistic, or artistic",
                s
            ))
        })?;
        request = request.with_style(image_style);
    }

    if let Some(n) = negative {
        request = request.with_negative_prompt(n);
    }

    if let Some(s) = seed {
        request = request.with_seed(s);
    }

    // Determine output path
    let output_path =
        output.unwrap_or_else(|| generate_output_path(&PathBuf::from("."), ImageFormat::Png));

    // Generate image
    let response = generate_image(&client, &request, Some(&output_path)).await?;

    info!(
        "Generated image: {} ({} bytes, {}ms)",
        output_path.display(),
        response.size_bytes(),
        response.generation_time_ms
    );

    Ok(output_path)
}

/// Transform an existing image with a prompt
pub async fn transform(
    input: PathBuf,
    prompt: String,
    output: Option<PathBuf>,
    strength: Option<f32>,
    model: Option<String>,
) -> Result<PathBuf> {
    let api_key = check_api_key_available()?;

    let client = ImageClient::new(api_key)?;

    // Validate input exists
    if !input.exists() {
        return Err(Error::ImageReadError(format!(
            "Input file not found: {}",
            input.display()
        )));
    }

    // Build request
    let mut request = TransformRequest::new(input.clone(), prompt);

    if let Some(m) = model {
        request = request.with_model(m);
    }

    if let Some(s) = strength {
        request = request.with_strength(s);
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let ext = input.extension().unwrap_or_default().to_string_lossy();
        let ext = if ext.is_empty() {
            "png".to_string()
        } else {
            ext.to_string()
        };
        input.with_file_name(format!("{}_transformed.{}", stem, ext))
    });

    // Transform image
    let response = transform_image(&client, &request, Some(&output_path)).await?;

    info!(
        "Transformed image: {} ({} bytes, {}ms)",
        output_path.display(),
        response.size_bytes(),
        response.generation_time_ms
    );

    Ok(output_path)
}

/// Upscale an image
pub async fn upscale(
    input: PathBuf,
    scale: u32,
    output: Option<PathBuf>,
    model: Option<String>,
) -> Result<PathBuf> {
    // API key might not be required for local upscaling
    let api_key = check_api_key_available().ok();

    // Validate input exists
    if !input.exists() {
        return Err(Error::ImageReadError(format!(
            "Input file not found: {}",
            input.display()
        )));
    }

    // Validate scale
    if !(1..=4).contains(&scale) {
        return Err(Error::InvalidInput(
            "Scale must be between 1 and 4".to_string(),
        ));
    }

    // Build request
    let mut request = UpscaleRequest::new(input.clone()).with_scale(scale);

    if let Some(m) = model {
        request = request.with_model(m);
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let ext = input.extension().unwrap_or_default().to_string_lossy();
        let ext = if ext.is_empty() {
            "png".to_string()
        } else {
            ext.to_string()
        };
        input.with_file_name(format!("{}_{}x.{}", stem, scale, ext))
    });

    // Create client if API key available, otherwise use local upscaling
    let response = if let Some(key) = api_key {
        let client = ImageClient::new(key)?;
        upscale_image(&client, &request, Some(&output_path)).await?
    } else {
        // Local upscaling only
        info!("No API key available, using local upscaling");
        let input_data = std::fs::read(&input)
            .map_err(|e| Error::ImageReadError(format!("{}: {}", input.display(), e)))?;

        let response = local_upscale_only(&input_data, scale)?;
        response
            .save_to_file(&output_path)
            .map_err(|e| Error::ImageSaveError(format!("{}: {}", output_path.display(), e)))?;
        response
    };

    info!(
        "Upscaled image: {} ({}x, {} bytes, {}ms)",
        output_path.display(),
        scale,
        response.size_bytes(),
        response.generation_time_ms
    );

    Ok(output_path)
}

/// Inpaint a region of an image
pub async fn inpaint(
    input: PathBuf,
    mask: PathBuf,
    prompt: String,
    output: Option<PathBuf>,
    model: Option<String>,
) -> Result<PathBuf> {
    let api_key = check_api_key_available()?;

    let client = ImageClient::new(api_key)?;

    // Validate inputs exist
    if !input.exists() {
        return Err(Error::ImageReadError(format!(
            "Input file not found: {}",
            input.display()
        )));
    }

    if !mask.exists() {
        return Err(Error::ImageReadError(format!(
            "Mask file not found: {}",
            mask.display()
        )));
    }

    // Build request
    let mut request = InpaintRequest::new(input.clone(), mask, prompt);

    if let Some(m) = model {
        request = request.with_model(m);
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        let ext = input.extension().unwrap_or_default().to_string_lossy();
        let ext = if ext.is_empty() {
            "png".to_string()
        } else {
            ext.to_string()
        };
        input.with_file_name(format!("{}_inpainted.{}", stem, ext))
    });

    // Inpaint image
    let response = inpaint_image(&client, &request, Some(&output_path)).await?;

    info!(
        "Inpainted image: {} ({} bytes, {}ms)",
        output_path.display(),
        response.size_bytes(),
        response.generation_time_ms
    );

    Ok(output_path)
}

/// List available image models
pub fn list_models() -> Vec<ModelInfo> {
    IMAGE_MODELS
        .iter()
        .map(|m| ModelInfo {
            id: m.id.to_string(),
            name: m.name.to_string(),
            description: m.description.to_string(),
            text_to_image: m.capabilities.text_to_image,
            image_to_image: m.capabilities.image_to_image,
            upscaling: m.capabilities.upscaling,
            inpainting: m.capabilities.inpainting,
            cost_per_image: m.cost_per_image,
        })
        .collect()
}

/// Model information for display
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub text_to_image: bool,
    pub image_to_image: bool,
    pub upscaling: bool,
    pub inpainting: bool,
    pub cost_per_image: Option<f64>,
}

impl ModelInfo {
    /// Format capabilities as a string
    pub fn capabilities_string(&self) -> String {
        let mut caps = Vec::new();
        if self.text_to_image {
            caps.push("text-to-image");
        }
        if self.image_to_image {
            caps.push("image-to-image");
        }
        if self.upscaling {
            caps.push("upscaling");
        }
        if self.inpainting {
            caps.push("inpainting");
        }
        caps.join(", ")
    }

    /// Format cost as a string
    pub fn cost_string(&self) -> String {
        self.cost_per_image
            .map(|c| format!("~${:.2}", c))
            .unwrap_or_else(|| "N/A".to_string())
    }
}

/// Local upscaling without API
fn local_upscale_only(input_data: &[u8], scale: u32) -> Result<crate::image::ImageResponse> {
    use std::io::Cursor;
    use std::time::Instant;

    let start = Instant::now();

    let img = image::load_from_memory(input_data)
        .map_err(|e| Error::ImageReadError(format!("Failed to decode image: {}", e)))?;

    let (width, height) = (img.width(), img.height());
    let new_width = width * scale;
    let new_height = height * scale;

    let resized = img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);

    // Encode to PNG using write_to
    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| Error::ImageSaveError(format!("Failed to encode image: {}", e)))?;

    let generation_time = start.elapsed().as_millis() as u64;

    Ok(crate::image::ImageResponse::new(
        output.into_inner(),
        ImageFormat::Png,
        "local/lanczos3".to_string(),
        generation_time,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_models() {
        let models = list_models();
        assert!(!models.is_empty());

        // Check that all models have required fields
        for model in &models {
            assert!(!model.id.is_empty());
            assert!(!model.name.is_empty());
            assert!(!model.description.is_empty());
        }
    }

    #[test]
    fn test_model_info_capabilities_string() {
        let model = ModelInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test model".to_string(),
            text_to_image: true,
            image_to_image: true,
            upscaling: false,
            inpainting: false,
            cost_per_image: Some(0.01),
        };

        let caps = model.capabilities_string();
        assert!(caps.contains("text-to-image"));
        assert!(caps.contains("image-to-image"));
        assert!(!caps.contains("upscaling"));
    }

    #[test]
    fn test_model_info_cost_string() {
        let model = ModelInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test model".to_string(),
            text_to_image: true,
            image_to_image: false,
            upscaling: false,
            inpainting: false,
            cost_per_image: Some(0.05),
        };

        assert_eq!(model.cost_string(), "~$0.05");

        let model_no_cost = ModelInfo {
            cost_per_image: None,
            ..model
        };

        assert_eq!(model_no_cost.cost_string(), "N/A");
    }
}
