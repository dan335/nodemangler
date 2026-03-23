/// AI-powered image operations using external APIs (OpenAI).
///
/// Provides three operations:
/// - **generate** — text prompt → generated image (DALL-E / gpt-image-1)
/// - **edit** — image + text prompt → edited image (OpenAI image edits)
/// - **variation** — image → variation image (OpenAI image variations)

pub mod shared;
pub mod generate;
pub mod edit;
pub mod variation;
