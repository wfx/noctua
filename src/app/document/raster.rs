// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/raster.rs
//
// Raster image document support (PNG, JPEG, WebP, etc.).

use std::path::Path;

use image::{imageops, DynamicImage, GenericImageView, ImageReader};

use super::{
    DocResult, DocumentInfo, FlipDirection, ImageHandle, Renderable, RenderOutput, Rotation,
    TransformState, Transformable,
};

/// Represents a raster image document (PNG, JPEG, WebP, ...).
pub struct RasterDocument {
    /// The decoded image document.
    document: DynamicImage,
    /// Native width (original, before transforms).
    native_width: u32,
    /// Native height (original, before transforms).
    native_height: u32,
    /// Current transformation state.
    transform: TransformState,
    /// Cached handle for rendering.
    pub handle: ImageHandle,
}

impl RasterDocument {
    /// Load a raster document from disk.
    pub fn open(path: &Path) -> image::ImageResult<Self> {
        let document = ImageReader::open(path)?.decode()?;
        let (native_width, native_height) = document.dimensions();
        let handle = super::create_image_handle_from_image(&document);

        Ok(Self {
            document,
            native_width,
            native_height,
            transform: TransformState::default(),
            handle,
        })
    }

    /// Rebuild the handle after mutating `document`.
    fn refresh_handle(&mut self) {
        self.handle = super::create_image_handle_from_image(&self.document);
    }

    /// Returns the current pixel dimensions (width, height) after transforms.
    pub fn dimensions(&self) -> (u32, u32) {
        self.document.dimensions()
    }

    /// Save the current document to disk.
    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> image::ImageResult<()> {
        self.document.save(path)
    }

    /// Extract metadata for this raster document.
    pub fn extract_meta(&self, path: &Path) -> super::meta::DocumentMeta {
        super::meta::build_raster_meta(path, &self.document, self.native_width, self.native_height)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Renderable for RasterDocument {
    fn render(&mut self, _scale: f64) -> DocResult<RenderOutput> {
        // Raster images don't re-render at different scales (lossy),
        // we just return the current handle.
        let (width, height) = self.dimensions();
        Ok(RenderOutput {
            handle: self.handle.clone(),
            width,
            height,
        })
    }

    fn info(&self) -> DocumentInfo {
        DocumentInfo {
            width: self.native_width,
            height: self.native_height,
            format: "Raster".to_string(),
        }
    }
}

impl Transformable for RasterDocument {
    fn rotate(&mut self, rotation: Rotation) {
        let current_deg = self.transform.rotation.to_degrees();
        let new_deg = rotation.to_degrees();
        let diff_deg = (new_deg - current_deg + 360) % 360;

        match diff_deg {
            0 => {}
            90 => {
                self.document = DynamicImage::ImageRgba8(imageops::rotate90(&self.document));
            }
            180 => {
                self.document = DynamicImage::ImageRgba8(imageops::rotate180(&self.document));
            }
            270 => {
                self.document = DynamicImage::ImageRgba8(imageops::rotate270(&self.document));
            }
            _ => unreachable!("Invalid rotation diff: {}", diff_deg),
        }
        self.transform.rotation = rotation;
        self.refresh_handle();
    }

    fn flip(&mut self, direction: FlipDirection) {
        match direction {
            FlipDirection::Horizontal => {
                self.document = DynamicImage::ImageRgba8(imageops::flip_horizontal(&self.document));
                self.transform.flip_h = !self.transform.flip_h;
            }
            FlipDirection::Vertical => {
                self.document = DynamicImage::ImageRgba8(imageops::flip_vertical(&self.document));
                self.transform.flip_v = !self.transform.flip_v;
            }
        }
        self.refresh_handle();
    }

    fn transform_state(&self) -> TransformState {
        self.transform
    }
}
