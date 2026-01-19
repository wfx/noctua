// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/vector.rs
//
// Vector documents (SVG, etc.).

use std::path::Path;

use image::{imageops, DynamicImage, RgbaImage};
use resvg::tiny_skia::{self, Pixmap};
use resvg::usvg::{Options, Tree};

use super::{
    DocResult, DocumentInfo, FlipDirection, ImageHandle, Renderable, RenderOutput, Rotation,
    TransformState, Transformable,
};
use crate::constant::MIN_PIXMAP_SIZE;

/// Represents a vector document such as SVG.
pub struct VectorDocument {
    /// Parsed SVG document for re-rendering at different scales.
    document: Tree,
    /// Native width of the SVG (from viewBox or width attribute).
    native_width: u32,
    /// Native height of the SVG (from viewBox or height attribute).
    native_height: u32,
    /// Current render scale (1.0 = native size).
    current_scale: f64,
    /// Accumulated transformations.
    transform: TransformState,
    /// Rasterized image at the current scale.
    pub rendered: DynamicImage,
    /// Image handle for display.
    pub handle: ImageHandle,
    /// Current rendered width.
    pub width: u32,
    /// Current rendered height.
    pub height: u32,
}

impl VectorDocument {
    /// Load a vector document from disk.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let raw_data = std::fs::read_to_string(path)?;

        // Parse SVG with default options.
        let options = Options::default();
        let document = Tree::from_str(&raw_data, &options)?;

        // Get native size from the parsed document.
        let size = document.size();
        let native_width = size.width().ceil() as u32;
        let native_height = size.height().ceil() as u32;

        let transform = TransformState::default();

        // Render at native scale (1.0).
        let (rendered, width, height) =
            render_document(&document, native_width, native_height, 1.0, &transform)?;
        let handle = super::create_image_handle_from_image(&rendered);

        Ok(Self {
            document,
            native_width,
            native_height,
            current_scale: 1.0,
            transform,
            rendered,
            handle,
            width,
            height,
        })
    }

    /// Returns the dimensions of the rasterized representation.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Re-render the SVG at a new scale, preserving transformations.
    /// Returns true if re-rendering occurred.
    #[allow(dead_code)]
    pub fn render_at_scale(&mut self, scale: f64) -> bool {
        // Skip if scale hasn't changed
        if (self.current_scale - scale).abs() < f64::EPSILON {
            return false;
        }

        match render_document(
            &self.document,
            self.native_width,
            self.native_height,
            scale,
            &self.transform,
        ) {
            Ok((rendered, width, height)) => {
                self.current_scale = scale;
                self.rendered = rendered;
                self.width = width;
                self.height = height;
                self.handle = super::create_image_handle_from_image(&self.rendered);
                true
            }
            Err(e) => {
                log::error!("Failed to re-render SVG at scale {}: {}", scale, e);
                false
            }
        }
    }

    /// Re-render with current scale and transform.
    fn rerender(&mut self) {
        if let Ok((rendered, width, height)) = render_document(
            &self.document,
            self.native_width,
            self.native_height,
            self.current_scale,
            &self.transform,
        ) {
            self.rendered = rendered;
            self.width = width;
            self.height = height;
            self.handle = super::create_image_handle_from_image(&self.rendered);
        }
    }

    /// Extract metadata for this vector document.
    pub fn extract_meta(&self, path: &Path) -> super::meta::DocumentMeta {
        // Report native dimensions in metadata.
        super::meta::build_vector_meta(path, self.native_width, self.native_height)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Renderable for VectorDocument {
    fn render(&mut self, scale: f64) -> DocResult<RenderOutput> {
        self.render_at_scale(scale);
        Ok(RenderOutput {
            handle: self.handle.clone(),
            width: self.width,
            height: self.height,
        })
    }

    fn info(&self) -> DocumentInfo {
        DocumentInfo {
            width: self.native_width,
            height: self.native_height,
            format: "SVG".to_string(),
        }
    }
}

impl Transformable for VectorDocument {
    fn rotate(&mut self, rotation: Rotation) {
        self.transform.rotation = rotation;
        self.rerender();
    }

    fn flip(&mut self, direction: FlipDirection) {
        match direction {
            FlipDirection::Horizontal => self.transform.flip_h = !self.transform.flip_h,
            FlipDirection::Vertical => self.transform.flip_v = !self.transform.flip_v,
        }
        self.rerender();
    }

    fn transform_state(&self) -> TransformState {
        self.transform
    }
}

/// Render the SVG document at a given scale with transformations.
fn render_document(
    document: &Tree,
    native_width: u32,
    native_height: u32,
    scale: f64,
    transform: &TransformState,
) -> anyhow::Result<(DynamicImage, u32, u32)> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let width = (((native_width as f64) * scale).ceil() as u32).max(MIN_PIXMAP_SIZE);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let height = (((native_height as f64) * scale).ceil() as u32).max(MIN_PIXMAP_SIZE);

    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;

    #[allow(clippy::cast_possible_truncation)]
    let scale_f32 = scale as f32;
    let ts = tiny_skia::Transform::from_scale(scale_f32, scale_f32);
    resvg::render(document, ts, &mut pixmap.as_mut());

    let mut image = pixmap_to_dynamic_image(&pixmap);

    // Apply flip transformations
    if transform.flip_h {
        image = DynamicImage::ImageRgba8(imageops::flip_horizontal(&image));
    }
    if transform.flip_v {
        image = DynamicImage::ImageRgba8(imageops::flip_vertical(&image));
    }

    // Apply rotation
    image = match transform.rotation {
        Rotation::Cw90 => DynamicImage::ImageRgba8(imageops::rotate90(&image)),
        Rotation::Cw180 => DynamicImage::ImageRgba8(imageops::rotate180(&image)),
        Rotation::Cw270 => DynamicImage::ImageRgba8(imageops::rotate270(&image)),
        Rotation::None => image,
    };

    let final_width = image.width();
    let final_height = image.height();

    Ok((image, final_width, final_height))
}

/// Convert a tiny_skia Pixmap to a DynamicImage.
fn pixmap_to_dynamic_image(pixmap: &Pixmap) -> DynamicImage {
    let width = pixmap.width();
    let height = pixmap.height();

    // tiny_skia uses premultiplied alpha, we need to unpremultiply for image crate
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for pixel in pixmap.pixels() {
        let a = pixel.alpha();
        if a == 0 {
            pixels.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // Unpremultiply: color = premultiplied_color * 255 / alpha
            let r = (pixel.red() as u16 * 255 / a as u16) as u8;
            let g = (pixel.green() as u16 * 255 / a as u16) as u8;
            let b = (pixel.blue() as u16 * 255 / a as u16) as u8;
            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    let rgba_image = RgbaImage::from_raw(width, height, pixels)
        .expect("Failed to create RgbaImage from pixmap data");

    DynamicImage::ImageRgba8(rgba_image)
}
