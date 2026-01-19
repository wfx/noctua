// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/mod.rs
//
// Document module root: common enums and type erasure for document kinds.

pub mod cache;
pub mod file;
pub mod meta;
pub mod portable;
pub mod raster;
pub mod utils;
pub mod vector;

use cosmic::iced_renderer::graphics::image::image_rs::ImageFormat as CosmicImageFormat;
use image::GenericImageView;
use std::fmt;
use std::path::Path;

use self::portable::PortableDocument;
use self::raster::RasterDocument;
use self::vector::VectorDocument;

// ============================================================================
// Type Definitions
// ============================================================================

/// Result type alias for document operations.
pub type DocResult<T> = anyhow::Result<T>;

/// Rotation state for documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    /// No rotation (0 degrees).
    #[default]
    None,
    /// 90 degrees clockwise.
    Cw90,
    /// 180 degrees.
    Cw180,
    /// 270 degrees clockwise (90 counter-clockwise).
    Cw270,
}

impl Rotation {
    /// Rotate clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_cw(self) -> Self {
        match self {
            Self::None => Self::Cw90,   // 0 → 90
            Self::Cw90 => Self::Cw180,  // 90 → 180
            Self::Cw180 => Self::Cw270, // 180 → 270
            Self::Cw270 => Self::None,  // 270 → 0
        }
    }

    /// Rotate counter-clockwise by 90 degrees.
    #[must_use]
    pub fn rotate_ccw(self) -> Self {
        match self {
            Self::None => Self::Cw270,  // 0 → 270
            Self::Cw270 => Self::Cw180, // 270 → 180
            Self::Cw180 => Self::Cw90,  // 180 → 90
            Self::Cw90 => Self::None,   // 90 → 0
        }
    }

    /// Convert to degrees (0, 90, 180, 270).
    #[must_use]
    pub fn to_degrees(self) -> i16 {
        match self {
            Self::None => 0,
            Self::Cw90 => 90,
            Self::Cw180 => 180,
            Self::Cw270 => 270,
        }
    }
}

/// Flip direction for documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipDirection {
    /// Flip along the vertical axis (mirror left-right).
    Horizontal,
    /// Flip along the horizontal axis (mirror top-bottom).
    Vertical,
}

/// Current transformation state of a document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransformState {
    /// Current rotation.
    pub rotation: Rotation,
    /// Whether flipped horizontally.
    pub flip_h: bool,
    /// Whether flipped vertically.
    pub flip_v: bool,
}



/// Output of a render operation.
///
/// Used as return type for the `Renderable::render()` trait method.
/// Not constructed externally - only returned by trait implementations.
#[allow(dead_code)]
pub struct RenderOutput {
    /// Image handle for display.
    pub handle: ImageHandle,
    /// Rendered width in pixels.
    pub width: u32,
    /// Rendered height in pixels.
    pub height: u32,
}

/// Document metadata/information.
///
/// Used as return type for the `Renderable::info()` trait method.
/// Contains native dimensions and format description before any transformations.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    /// Native width in pixels (before transforms).
    pub width: u32,
    /// Native height in pixels (before transforms).
    pub height: u32,
    /// Document format description.
    pub format: String,
}

// ============================================================================
// Traits
// ============================================================================

/// Trait for documents that can be rendered to an image.
///
/// This trait is used internally through type erasure via `DocumentContent`.
/// The UI layer calls methods on `DocumentContent`, which delegates to the
/// specific document type implementations (Raster, Vector, Portable).
#[allow(dead_code)]
pub trait Renderable {
    /// Render the document at the given scale factor.
    fn render(&mut self, scale: f64) -> DocResult<RenderOutput>;

    /// Get document information (dimensions, format).
    fn info(&self) -> DocumentInfo;
}

/// Trait for documents that support geometric transformations.
pub trait Transformable {
    /// Apply a rotation state.
    fn rotate(&mut self, rotation: Rotation);

    /// Flip in the given direction.
    fn flip(&mut self, direction: FlipDirection);

    /// Get the current transformation state.
    fn transform_state(&self) -> TransformState;
}

/// Trait for documents with multiple pages.
pub trait MultiPage {
    /// Get total number of pages.
    fn page_count(&self) -> usize;

    /// Get current page index (0-based).
    fn current_page(&self) -> usize;

    /// Navigate to a specific page.
    fn go_to_page(&mut self, page: usize) -> DocResult<()>;
}

/// Trait for multi-page documents that support thumbnail generation.
///
/// Currently implemented only by `PortableDocument` (PDF).
/// Methods are called through `DocumentContent` type erasure.
#[allow(dead_code)]
pub trait MultiPageThumbnails: MultiPage {
    /// Get cached thumbnail for a page, if available.
    fn get_thumbnail(&self, page: usize) -> Option<ImageHandle>;

    /// Check if all thumbnails are ready.
    fn thumbnails_ready(&self) -> bool;

    /// Get count of thumbnails currently loaded.
    fn thumbnails_loaded(&self) -> usize;

    /// Generate thumbnail for a single page. Returns next page to generate.
    fn generate_thumbnail_page(&mut self, page: usize) -> Option<usize>;

    /// Generate all thumbnails (blocking).
    fn generate_all_thumbnails(&mut self);
}

// ============================================================================
// Document Types
// ============================================================================

/// Supported document kinds (for format detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Raster,
    Vector,
    Portable,
}

impl DocumentKind {
    /// Detect document kind from file path.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();

        // SVG
        if ext == "svg" || ext == "svgz" {
            return Some(Self::Vector);
        }

        // PDF
        if ext == "pdf" {
            return Some(Self::Portable);
        }

        // Raster: Check via cosmic/image-rs
        if CosmicImageFormat::from_path(path).is_ok() {
            return Some(Self::Raster);
        }

        None
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raster => write!(f, "Raster"),
            Self::Vector => write!(f, "Vector"),
            Self::Portable => write!(f, "Portable"),
        }
    }
}

// ============================================================================
// Image Handle Helper
// ============================================================================

/// Handle for rendered images (compatible with cosmic/iced).
pub type ImageHandle = cosmic::widget::image::Handle;

/// Create an image handle from RGBA pixel data.
#[must_use]
pub fn create_image_handle(pixels: Vec<u8>, width: u32, height: u32) -> ImageHandle {
    cosmic::widget::image::Handle::from_rgba(width, height, pixels)
}

/// Create an image handle from a DynamicImage.
#[must_use]
pub fn create_image_handle_from_image(img: &image::DynamicImage) -> ImageHandle {
    let (width, height) = img.dimensions();
    let pixels = img.to_rgba8().into_raw();
    create_image_handle(pixels, width, height)
}

// ============================================================================
// Document Content Enum
// ============================================================================

/// Type-erased document content.
///
/// The application only holds one document at a time, so the size difference
/// between variants (536 bytes for Vector vs 184 bytes for Portable) is acceptable.
/// Boxing would add unnecessary indirection without measurable performance benefit.
#[allow(clippy::large_enum_variant)]
pub enum DocumentContent {
    Raster(RasterDocument),
    Vector(VectorDocument),
    Portable(PortableDocument),
}

impl fmt::Debug for DocumentContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raster(_) => write!(f, "DocumentContent::Raster(...)"),
            Self::Vector(_) => write!(f, "DocumentContent::Vector(...)"),
            Self::Portable(_) => write!(f, "DocumentContent::Portable(...)"),
        }
    }
}

// ============================================================================
// Trait Implementations for DocumentContent
// ============================================================================

impl Renderable for DocumentContent {
    fn render(&mut self, scale: f64) -> DocResult<RenderOutput> {
        match self {
            Self::Raster(doc) => doc.render(scale),
            Self::Vector(doc) => doc.render(scale),
            Self::Portable(doc) => doc.render(scale),
        }
    }

    fn info(&self) -> DocumentInfo {
        match self {
            Self::Raster(doc) => doc.info(),
            Self::Vector(doc) => doc.info(),
            Self::Portable(doc) => doc.info(),
        }
    }
}

impl Transformable for DocumentContent {
    fn rotate(&mut self, rotation: Rotation) {
        match self {
            Self::Raster(doc) => doc.rotate(rotation),
            Self::Vector(doc) => doc.rotate(rotation),
            Self::Portable(doc) => doc.rotate(rotation),
        }
    }

    fn flip(&mut self, direction: FlipDirection) {
        match self {
            Self::Raster(doc) => doc.flip(direction),
            Self::Vector(doc) => doc.flip(direction),
            Self::Portable(doc) => doc.flip(direction),
        }
    }

    fn transform_state(&self) -> TransformState {
        match self {
            Self::Raster(doc) => doc.transform_state(),
            Self::Vector(doc) => doc.transform_state(),
            Self::Portable(doc) => doc.transform_state(),
        }
    }
}

// ============================================================================
// Convenience Methods for DocumentContent
// ============================================================================

impl DocumentContent {
    /// Rotate document 90 degrees clockwise.
    pub fn rotate_cw(&mut self) {
        let new_rotation = self.transform_state().rotation.rotate_cw();
        self.rotate(new_rotation);
    }

    /// Rotate document 90 degrees counter-clockwise.
    pub fn rotate_ccw(&mut self) {
        let new_rotation = self.transform_state().rotation.rotate_ccw();
        self.rotate(new_rotation);
    }

    /// Flip document horizontally.
    pub fn flip_horizontal(&mut self) {
        self.flip(FlipDirection::Horizontal);
    }

    /// Flip document vertically.
    pub fn flip_vertical(&mut self) {
        self.flip(FlipDirection::Vertical);
    }

    /// Get document kind.
    ///
    /// Reserved for future use (format-specific optimizations, statistics).
    #[allow(dead_code)]
    #[must_use]
    pub fn kind(&self) -> DocumentKind {
        match self {
            Self::Raster(_) => DocumentKind::Raster,
            Self::Vector(_) => DocumentKind::Vector,
            Self::Portable(_) => DocumentKind::Portable,
        }
    }

    /// Check if this document supports multiple pages.
    #[must_use]
    pub fn is_multi_page(&self) -> bool {
        self.page_count().is_some_and(|n| n > 1)
    }

    /// Get page count if applicable.
    #[must_use]
    pub fn page_count(&self) -> Option<usize> {
        match self {
            Self::Portable(doc) => Some(doc.page_count()),
            _ => None,
        }
    }

    /// Get current page index if applicable.
    #[must_use]
    pub fn current_page(&self) -> Option<usize> {
        match self {
            Self::Portable(doc) => Some(doc.current_page()),
            _ => None,
        }
    }

    /// Navigate to a specific page.
    pub fn go_to_page(&mut self, page: usize) -> DocResult<()> {
        match self {
            Self::Portable(doc) => doc.go_to_page(page),
            _ => Err(anyhow::anyhow!("Document does not support multiple pages")),
        }
    }

    /// Get cached thumbnail for a page.
    #[must_use]
    pub fn get_thumbnail(&self, page: usize) -> Option<ImageHandle> {
        match self {
            Self::Portable(doc) => doc.get_thumbnail(page),
            _ => None,
        }
    }

    /// Check if thumbnails are ready.
    #[must_use]
    pub fn thumbnails_ready(&self) -> bool {
        match self {
            Self::Portable(doc) => doc.thumbnails_ready(),
            _ => false,
        }
    }

    /// Get count of loaded thumbnails.
    #[must_use]
    pub fn thumbnails_loaded(&self) -> usize {
        match self {
            Self::Portable(doc) => doc.thumbnails_loaded(),
            _ => 0,
        }
    }

    /// Generate thumbnail for a single page.
    pub fn generate_thumbnail_page(&mut self, page: usize) -> Option<usize> {
        match self {
            Self::Portable(doc) => doc.generate_thumbnail_page(page),
            _ => None,
        }
    }

    /// Generate all thumbnails (blocking).
    ///
    /// Convenience wrapper for `MultiPageThumbnails::generate_all_thumbnails()`.
    /// Currently unused - thumbnails are generated incrementally via `generate_thumbnail_page()`.
    #[allow(dead_code)]
    pub fn generate_thumbnails(&mut self) {
        if let Self::Portable(doc) = self { doc.generate_all_thumbnails() }
    }

    /// Get current image handle for display.
    #[must_use]
    pub fn handle(&self) -> ImageHandle {
        match self {
            Self::Raster(doc) => doc.handle.clone(),
            Self::Vector(doc) => doc.handle.clone(),
            Self::Portable(doc) => doc.handle.clone(),
        }
    }

    /// Get current document dimensions.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Raster(doc) => doc.dimensions(),
            Self::Vector(doc) => doc.dimensions(),
            Self::Portable(doc) => doc.dimensions(),
        }
    }

    /// Extract document metadata.
    pub fn extract_meta(&self, path: &Path) -> meta::DocumentMeta {
        match self {
            Self::Raster(doc) => doc.extract_meta(path),
            Self::Vector(doc) => doc.extract_meta(path),
            Self::Portable(doc) => doc.extract_meta(path),
        }
    }
}

// ============================================================================
// Public Utilities
// ============================================================================

/// Set an image file as desktop wallpaper.
pub fn set_as_wallpaper(path: &Path) {
    utils::set_as_wallpaper(path);
}
