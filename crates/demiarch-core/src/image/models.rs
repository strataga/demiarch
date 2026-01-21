//! Image generation model definitions
//!
//! Defines available models for image generation via OpenRouter.

use serde::{Deserialize, Serialize};

/// Information about an image generation model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageModel {
    /// Model identifier (OpenRouter format)
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// Brief description of the model
    pub description: &'static str,
    /// Supported capabilities
    pub capabilities: ModelCapabilities,
    /// Approximate cost per image (USD)
    pub cost_per_image: Option<f64>,
}

/// Capabilities supported by a model
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    /// Can generate images from text
    pub text_to_image: bool,
    /// Can transform existing images
    pub image_to_image: bool,
    /// Can upscale images
    pub upscaling: bool,
    /// Can do inpainting (masked editing)
    pub inpainting: bool,
}

impl ModelCapabilities {
    /// Create capabilities for text-to-image only
    pub const fn text_only() -> Self {
        Self {
            text_to_image: true,
            image_to_image: false,
            upscaling: false,
            inpainting: false,
        }
    }

    /// Create capabilities for full suite
    pub const fn full() -> Self {
        Self {
            text_to_image: true,
            image_to_image: true,
            upscaling: true,
            inpainting: true,
        }
    }

    /// Create capabilities for generation without inpainting
    pub const fn generation() -> Self {
        Self {
            text_to_image: true,
            image_to_image: true,
            upscaling: false,
            inpainting: false,
        }
    }
}

/// Available image generation models
pub static IMAGE_MODELS: &[ImageModel] = &[
    ImageModel {
        id: "google/gemini-2.0-flash-exp:image",
        name: "Gemini 2.0 Flash (Image)",
        description: "Google's fast, efficient image generation model. Good quality with quick generation times.",
        capabilities: ModelCapabilities::generation(),
        cost_per_image: Some(0.01),
    },
    ImageModel {
        id: "nano-banana-pro",
        name: "Nano Banana Pro",
        description: "High-quality image generation with excellent detail and artistic capabilities.",
        capabilities: ModelCapabilities::full(),
        cost_per_image: Some(0.02),
    },
    ImageModel {
        id: "openai/dall-e-3",
        name: "DALL-E 3",
        description: "OpenAI's flagship image model. Excellent at following complex prompts and artistic styles.",
        capabilities: ModelCapabilities::text_only(),
        cost_per_image: Some(0.04),
    },
    ImageModel {
        id: "openai/dall-e-2",
        name: "DALL-E 2",
        description: "OpenAI's previous generation model. Good for variations and editing.",
        capabilities: ModelCapabilities {
            text_to_image: true,
            image_to_image: true,
            upscaling: false,
            inpainting: true,
        },
        cost_per_image: Some(0.02),
    },
    ImageModel {
        id: "stabilityai/stable-diffusion-xl",
        name: "Stable Diffusion XL",
        description: "Open-source model with excellent customization. Supports negative prompts well.",
        capabilities: ModelCapabilities::generation(),
        cost_per_image: Some(0.01),
    },
];

impl ImageModel {
    /// Get a model by ID
    pub fn by_id(id: &str) -> Option<&'static ImageModel> {
        IMAGE_MODELS.iter().find(|m| m.id == id)
    }

    /// Get the default model for text-to-image
    pub fn default_text_to_image() -> &'static ImageModel {
        IMAGE_MODELS
            .iter()
            .find(|m| m.id == "google/gemini-2.0-flash-exp:image")
            .unwrap_or(&IMAGE_MODELS[0])
    }

    /// Get models with a specific capability
    pub fn with_capability(
        filter: impl Fn(&ModelCapabilities) -> bool,
    ) -> impl Iterator<Item = &'static ImageModel> {
        IMAGE_MODELS.iter().filter(move |m| filter(&m.capabilities))
    }

    /// Check if this model supports a specific operation
    pub fn supports_text_to_image(&self) -> bool {
        self.capabilities.text_to_image
    }

    /// Check if this model supports image-to-image
    pub fn supports_image_to_image(&self) -> bool {
        self.capabilities.image_to_image
    }

    /// Check if this model supports upscaling
    pub fn supports_upscaling(&self) -> bool {
        self.capabilities.upscaling
    }

    /// Check if this model supports inpainting
    pub fn supports_inpainting(&self) -> bool {
        self.capabilities.inpainting
    }
}

/// Get fallback model IDs in order of preference
pub fn fallback_model_ids() -> &'static [&'static str] {
    &["nano-banana-pro", "openai/dall-e-3", "stabilityai/stable-diffusion-xl"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_lookup() {
        let model = ImageModel::by_id("google/gemini-2.0-flash-exp:image");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "Gemini 2.0 Flash (Image)");

        let model = ImageModel::by_id("nonexistent");
        assert!(model.is_none());
    }

    #[test]
    fn test_default_model() {
        let model = ImageModel::default_text_to_image();
        assert!(model.supports_text_to_image());
    }

    #[test]
    fn test_capability_filter() {
        let inpainting_models: Vec<_> =
            ImageModel::with_capability(|c| c.inpainting).collect();
        assert!(!inpainting_models.is_empty());
        for model in inpainting_models {
            assert!(model.supports_inpainting());
        }
    }

    #[test]
    fn test_capabilities() {
        let full = ModelCapabilities::full();
        assert!(full.text_to_image);
        assert!(full.image_to_image);
        assert!(full.upscaling);
        assert!(full.inpainting);

        let text_only = ModelCapabilities::text_only();
        assert!(text_only.text_to_image);
        assert!(!text_only.image_to_image);
        assert!(!text_only.upscaling);
        assert!(!text_only.inpainting);
    }
}
