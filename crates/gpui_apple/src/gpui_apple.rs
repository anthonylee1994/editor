//! Shared Apple rendering, task dispatch, and system services for GPUI.
//!
//! This crate renders GPUI scenes directly with Metal on Apple platforms. It
//! owns GPU resources and text rendering
//! while leaving application lifecycle, windowing, and input to each platform backend.

mod dispatcher;
pub mod display_link;
mod metal_atlas;
pub mod metal_renderer;

mod font;
mod open_type;
mod text_system;

pub use dispatcher::AppleDispatcher;
pub use text_system::AppleTextSystem;
