//! Image generation module for Demiarch
//!
//! Provides image generation capabilities using OpenRouter's API with models
//! that support image output (Nano Banana Pro, Gemini image models).
//!
//! Features:
//! - Text-to-image generation
//! - Image-to-image transformation
//! - Image upscaling
//! - Image inpainting

mod client;
mod models;
pub mod operations;
mod types;

pub use client::{ImageClient, ImageClientBuilder};
pub use models::{ImageModel, IMAGE_MODELS};
pub use operations::{generate_image, inpaint_image, transform_image, upscale_image};
pub use types::{
    ImageFormat, ImageRequest, ImageResponse, ImageSize, ImageStyle, InpaintRequest,
    TransformRequest, UpscaleRequest,
};
