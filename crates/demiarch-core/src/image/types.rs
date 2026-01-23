//! Image generation types
//!
//! Request and response types for image generation operations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Image size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageSize {
    /// 1024x1024 square image
    #[default]
    Square1024,
    /// 1024x1536 portrait image
    Portrait,
    /// 1536x1024 landscape image
    Landscape,
    /// Custom dimensions
    #[serde(skip)]
    Custom(u32, u32),
}

impl ImageSize {
    /// Get width and height as a tuple
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Square1024 => (1024, 1024),
            Self::Portrait => (1024, 1536),
            Self::Landscape => (1536, 1024),
            Self::Custom(w, h) => (*w, *h),
        }
    }

    /// Parse from string (e.g., "square", "portrait", "landscape", "1024x768")
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "square" | "square1024" | "1024x1024" => Some(Self::Square1024),
            "portrait" | "1024x1536" => Some(Self::Portrait),
            "landscape" | "1536x1024" => Some(Self::Landscape),
            s if s.contains('x') => {
                let parts: Vec<&str> = s.split('x').collect();
                if parts.len() == 2 {
                    let w = parts[0].parse().ok()?;
                    let h = parts[1].parse().ok()?;
                    Some(Self::Custom(w, h))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for ImageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (w, h) = self.dimensions();
        write!(f, "{}x{}", w, h)
    }
}

/// Image style presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageStyle {
    /// Vivid, colorful style
    #[default]
    Vivid,
    /// Natural, realistic style
    Natural,
    /// Photorealistic rendering
    Photorealistic,
    /// Artistic, creative style
    Artistic,
}

impl ImageStyle {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vivid" => Some(Self::Vivid),
            "natural" => Some(Self::Natural),
            "photorealistic" | "photo" => Some(Self::Photorealistic),
            "artistic" | "art" => Some(Self::Artistic),
            _ => None,
        }
    }

    /// Get style description for prompt enhancement
    pub fn prompt_modifier(&self) -> &'static str {
        match self {
            Self::Vivid => "vibrant colors, high contrast, dynamic",
            Self::Natural => "natural lighting, realistic colors",
            Self::Photorealistic => "photorealistic, 4K, detailed, professional photography",
            Self::Artistic => "artistic, creative, stylized",
        }
    }
}

impl std::fmt::Display for ImageStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vivid => write!(f, "vivid"),
            Self::Natural => write!(f, "natural"),
            Self::Photorealistic => write!(f, "photorealistic"),
            Self::Artistic => write!(f, "artistic"),
        }
    }
}

/// Image output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// PNG format (lossless)
    #[default]
    Png,
    /// JPEG format (lossy, smaller)
    Jpeg,
    /// WebP format (modern, efficient)
    WebP,
}

impl ImageFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
        }
    }

    /// Get MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
        }
    }

    /// Parse from string or file extension
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::WebP),
            _ => None,
        }
    }

    /// Detect format from file path
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::parse)
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

/// Request for text-to-image generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    /// Text description of the image to generate
    pub prompt: String,
    /// Model to use for generation
    #[serde(default = "default_model")]
    pub model: String,
    /// Desired image size
    #[serde(default)]
    pub size: ImageSize,
    /// Image style
    #[serde(default)]
    pub style: Option<ImageStyle>,
    /// Negative prompt (what to avoid)
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Seed for reproducibility
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_model() -> String {
    "google/gemini-2.0-flash-exp:image".to_string()
}

impl ImageRequest {
    /// Create a new image request with the given prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: default_model(),
            size: ImageSize::default(),
            style: None,
            negative_prompt: None,
            seed: None,
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the image size
    pub fn with_size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }

    /// Set the image style
    pub fn with_style(mut self, style: ImageStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set the negative prompt
    pub fn with_negative_prompt(mut self, negative: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative.into());
        self
    }

    /// Set the seed for reproducibility
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Build the enhanced prompt with style modifiers
    pub fn build_prompt(&self) -> String {
        let mut prompt = self.prompt.clone();

        if let Some(style) = &self.style {
            prompt = format!("{}, {}", prompt, style.prompt_modifier());
        }

        if let Some(negative) = &self.negative_prompt {
            prompt = format!("{}. Avoid: {}", prompt, negative);
        }

        prompt
    }
}

/// Request for image-to-image transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRequest {
    /// Path to the input image
    pub input_image: PathBuf,
    /// Text description of the transformation
    pub prompt: String,
    /// Model to use for transformation
    #[serde(default = "default_model")]
    pub model: String,
    /// Transformation strength (0.0-1.0, how much to change)
    #[serde(default = "default_strength")]
    pub strength: f32,
    /// Desired output size (defaults to input size)
    #[serde(default)]
    pub size: Option<ImageSize>,
}

fn default_strength() -> f32 {
    0.75
}

impl TransformRequest {
    /// Create a new transform request
    pub fn new(input_image: PathBuf, prompt: impl Into<String>) -> Self {
        Self {
            input_image,
            prompt: prompt.into(),
            model: default_model(),
            strength: default_strength(),
            size: None,
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the transformation strength
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Set the output size
    pub fn with_size(mut self, size: ImageSize) -> Self {
        self.size = Some(size);
        self
    }
}

/// Request for image upscaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaleRequest {
    /// Path to the input image
    pub input_image: PathBuf,
    /// Scale factor (2 or 4)
    #[serde(default = "default_scale")]
    pub scale: u32,
    /// Model to use for upscaling (if available)
    #[serde(default)]
    pub model: Option<String>,
}

fn default_scale() -> u32 {
    2
}

impl UpscaleRequest {
    /// Create a new upscale request
    pub fn new(input_image: PathBuf) -> Self {
        Self {
            input_image,
            scale: default_scale(),
            model: None,
        }
    }

    /// Set the scale factor
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.clamp(1, 4);
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Request for inpainting (masked image editing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InpaintRequest {
    /// Path to the input image
    pub input_image: PathBuf,
    /// Path to the mask image (white = edit, black = keep)
    pub mask_image: PathBuf,
    /// Text description of what to fill in the masked area
    pub prompt: String,
    /// Model to use for inpainting
    #[serde(default = "default_model")]
    pub model: String,
}

impl InpaintRequest {
    /// Create a new inpaint request
    pub fn new(input_image: PathBuf, mask_image: PathBuf, prompt: impl Into<String>) -> Self {
        Self {
            input_image,
            mask_image,
            prompt: prompt.into(),
            model: default_model(),
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// Response from image generation
#[derive(Debug, Clone)]
pub struct ImageResponse {
    /// Raw image bytes
    pub image_data: Vec<u8>,
    /// Image format
    pub format: ImageFormat,
    /// Model's interpretation of the prompt (if available)
    pub revised_prompt: Option<String>,
    /// Model that generated the image
    pub model_used: String,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
}

impl ImageResponse {
    /// Create a new image response
    pub fn new(
        image_data: Vec<u8>,
        format: ImageFormat,
        model_used: String,
        generation_time_ms: u64,
    ) -> Self {
        Self {
            image_data,
            format,
            revised_prompt: None,
            model_used,
            generation_time_ms,
        }
    }

    /// Set the revised prompt
    pub fn with_revised_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.revised_prompt = Some(prompt.into());
        self
    }

    /// Save the image to a file
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, &self.image_data)
    }

    /// Get the image size in bytes
    pub fn size_bytes(&self) -> usize {
        self.image_data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_size_dimensions() {
        assert_eq!(ImageSize::Square1024.dimensions(), (1024, 1024));
        assert_eq!(ImageSize::Portrait.dimensions(), (1024, 1536));
        assert_eq!(ImageSize::Landscape.dimensions(), (1536, 1024));
        assert_eq!(ImageSize::Custom(800, 600).dimensions(), (800, 600));
    }

    #[test]
    fn test_image_size_from_str() {
        assert_eq!(ImageSize::parse("square"), Some(ImageSize::Square1024));
        assert_eq!(ImageSize::parse("portrait"), Some(ImageSize::Portrait));
        assert_eq!(ImageSize::parse("landscape"), Some(ImageSize::Landscape));
        assert_eq!(
            ImageSize::parse("800x600"),
            Some(ImageSize::Custom(800, 600))
        );
        assert_eq!(ImageSize::parse("invalid"), None);
    }

    #[test]
    fn test_image_style_from_str() {
        assert_eq!(ImageStyle::parse("vivid"), Some(ImageStyle::Vivid));
        assert_eq!(ImageStyle::parse("natural"), Some(ImageStyle::Natural));
        assert_eq!(
            ImageStyle::parse("photo"),
            Some(ImageStyle::Photorealistic)
        );
        assert_eq!(ImageStyle::parse("art"), Some(ImageStyle::Artistic));
        assert_eq!(ImageStyle::parse("invalid"), None);
    }

    #[test]
    fn test_image_format() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::WebP.extension(), "webp");

        assert_eq!(ImageFormat::parse("png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::parse("jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::parse("webp"), Some(ImageFormat::WebP));
    }

    #[test]
    fn test_image_request_builder() {
        let request = ImageRequest::new("A sunset over mountains")
            .with_model("test-model")
            .with_size(ImageSize::Landscape)
            .with_style(ImageStyle::Vivid)
            .with_negative_prompt("blurry")
            .with_seed(42);

        assert_eq!(request.prompt, "A sunset over mountains");
        assert_eq!(request.model, "test-model");
        assert_eq!(request.size, ImageSize::Landscape);
        assert_eq!(request.style, Some(ImageStyle::Vivid));
        assert_eq!(request.negative_prompt, Some("blurry".to_string()));
        assert_eq!(request.seed, Some(42));
    }

    #[test]
    fn test_build_prompt_with_style() {
        let request = ImageRequest::new("A cat").with_style(ImageStyle::Vivid);

        let prompt = request.build_prompt();
        assert!(prompt.contains("A cat"));
        assert!(prompt.contains("vibrant colors"));
    }

    #[test]
    fn test_transform_request_strength_clamping() {
        let request = TransformRequest::new(PathBuf::from("test.png"), "transform").with_strength(2.0);
        assert_eq!(request.strength, 1.0);

        let request = TransformRequest::new(PathBuf::from("test.png"), "transform").with_strength(-0.5);
        assert_eq!(request.strength, 0.0);
    }
}
