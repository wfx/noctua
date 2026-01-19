// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/portable.rs
//
// Portable documents (PDF) with poppler backend.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use cairo::{Context, Format, ImageSurface};
use image::{imageops, DynamicImage, ImageReader};
use poppler::PopplerDocument;

use super::{
    cache, DocResult, DocumentInfo, FlipDirection, ImageHandle, MultiPage, MultiPageThumbnails,
    Renderable, RenderOutput, Rotation, TransformState, Transformable,
};
use crate::constant::{PDF_RENDER_QUALITY, PDF_THUMBNAIL_SIZE};

/// Represents a portable document (PDF).
pub struct PortableDocument {
    /// The parsed PDF document.
    document: PopplerDocument,
    /// Path to the source file (for caching).
    source_path: PathBuf,
    /// Total number of pages.
    num_pages: usize,
    /// Current page index (0-based).
    page_index: usize,
    /// Current transformation state.
    transform: TransformState,
    /// Current rendered page as image.
    pub rendered: DynamicImage,
    /// Image handle for display.
    pub handle: ImageHandle,
    /// Cached thumbnail handles for each page (None = not yet generated).
    thumbnail_cache: Option<Vec<ImageHandle>>,
}

impl PortableDocument {
    /// Open a PDF document and render the first page.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let document = PopplerDocument::new_from_file(path, None)
            .map_err(|e| anyhow::anyhow!("Failed to parse PDF: {}", e))?;

        let num_pages = document.get_n_pages();
        if num_pages == 0 {
            return Err(anyhow::anyhow!("PDF has no pages"));
        }

        let rendered = Self::render_page(&document, 0, Rotation::None)?;
        let handle = super::create_image_handle_from_image(&rendered);

        Ok(Self {
            document,
            source_path: path.to_path_buf(),
            num_pages,
            page_index: 0,
            transform: TransformState::default(),
            rendered,
            handle,
            thumbnail_cache: None,
        })
    }

    /// Get the number of thumbnails currently loaded.
    pub fn thumbnails_loaded(&self) -> usize {
        self.thumbnail_cache.as_ref().map_or(0, Vec::len)
    }

    /// Initialize thumbnail cache (empty, ready for incremental loading).
    fn init_thumbnail_cache(&mut self) {
        if self.thumbnail_cache.is_none() {
            self.thumbnail_cache = Some(Vec::with_capacity(self.num_pages));
        }
    }

    /// Generate a single thumbnail page. Returns the next page to generate, or None if done.
    pub fn generate_thumbnail_page(&mut self, page: usize) -> Option<usize> {
        // Initialize cache if needed.
        self.init_thumbnail_cache();

        // Check if we should generate this page.
        let should_generate = {
            let cache = self.thumbnail_cache.as_ref()?;
            page >= cache.len() && page < self.num_pages
        };

        if should_generate {
            let handle = self.load_or_generate_thumbnail(page);
            if let Some(cache) = self.thumbnail_cache.as_mut() {
                cache.push(handle);
            }
        }

        // Return next page if not done.
        let next = page + 1;
        if next < self.num_pages {
            Some(next)
        } else {
            None
        }
    }

    /// Load thumbnail from cache or generate and cache it.
    fn load_or_generate_thumbnail(&self, page: usize) -> ImageHandle {
        if let Some(handle) = cache::load_thumbnail(&self.source_path, page) {
            return handle;
        }

        match Self::render_page_at_scale(&self.document, page, Rotation::None, PDF_THUMBNAIL_SIZE)
        {
            Ok(img) => {
                let _ = cache::save_thumbnail(&self.source_path, page, &img);
                super::create_image_handle_from_image(&img)
            }
            Err(e) => {
                log::warn!("Failed to generate thumbnail for page {}: {}", page, e);
                ImageHandle::from_rgba(1, 1, vec![0, 0, 0, 0])
            }
        }
    }

    /// Render a specific page from the document to an image.
    fn render_page(
        document: &PopplerDocument,
        page_index: usize,
        rotation: Rotation,
    ) -> anyhow::Result<DynamicImage> {
        Self::render_page_at_scale(document, page_index, rotation, PDF_RENDER_QUALITY)
    }

    /// Render a specific page at a given scale.
    fn render_page_at_scale(
        document: &PopplerDocument,
        page_index: usize,
        rotation: Rotation,
        scale: f64,
    ) -> anyhow::Result<DynamicImage> {
        let page = document
            .get_page(page_index)
            .ok_or_else(|| anyhow::anyhow!("Failed to get page {}", page_index))?;

        let (page_width, page_height) = page.get_size();
        let rotation_degrees = rotation.to_degrees();

        let (width, height) = if rotation_degrees == 90 || rotation_degrees == 270 {
            (page_height, page_width)
        } else {
            (page_width, page_height)
        };

        #[allow(clippy::cast_possible_truncation)]
        let scaled_width = (width * scale) as i32;
        #[allow(clippy::cast_possible_truncation)]
        let scaled_height = (height * scale) as i32;

        let surface = ImageSurface::create(Format::ARgb32, scaled_width, scaled_height)
            .map_err(|e| anyhow::anyhow!("Failed to create Cairo surface: {}", e))?;

        let context = Context::new(&surface)
            .map_err(|e| anyhow::anyhow!("Failed to create Cairo context: {}", e))?;

        // Fill with white background.
        context.set_source_rgb(1.0, 1.0, 1.0);
        let _ = context.paint();

        context.scale(scale, scale);

        if rotation != Rotation::None {
            let center_x = width / 2.0;
            let center_y = height / 2.0;
            context.translate(center_x, center_y);
            context.rotate(f64::from(rotation_degrees) * std::f64::consts::PI / 180.0);
            context.translate(-page_width / 2.0, -page_height / 2.0);
        }

        page.render(&context);

        drop(context);
        surface.flush();

        let mut png_data: Vec<u8> = Vec::new();
        surface
            .write_to_png(&mut png_data)
            .map_err(|e| anyhow::anyhow!("Failed to write PNG: {}", e))?;

        let image = ImageReader::new(Cursor::new(png_data))
            .with_guessed_format()
            .map_err(|e| anyhow::anyhow!("Failed to read PNG format: {}", e))?
            .decode()
            .map_err(|e| anyhow::anyhow!("Failed to decode PNG: {}", e))?;

        Ok(image)
    }

    /// Re-render the current page with current transform.
    fn rerender(&mut self) {
        match Self::render_page(&self.document, self.page_index, self.transform.rotation) {
            Ok(mut rendered) => {
                // Apply flip transformations to the rendered result
                if self.transform.flip_h {
                    rendered = DynamicImage::ImageRgba8(imageops::flip_horizontal(&rendered));
                }
                if self.transform.flip_v {
                    rendered = DynamicImage::ImageRgba8(imageops::flip_vertical(&rendered));
                }
                self.rendered = rendered;
                self.refresh_handle();
            }
            Err(e) => {
                log::error!("Failed to render PDF page: {}", e);
            }
        }
    }

    /// Rebuild the handle after mutating `rendered`.
    fn refresh_handle(&mut self) {
        self.handle = super::create_image_handle_from_image(&self.rendered);
    }

    /// Returns the dimensions of the currently rendered page.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.rendered.width(), self.rendered.height())
    }

    /// Navigate to the next page.
    #[allow(dead_code)]
    pub fn next_page(&mut self) -> bool {
        if self.page_index + 1 < self.num_pages {
            self.page_index += 1;
            self.rerender();
            true
        } else {
            false
        }
    }

    /// Navigate to the previous page.
    #[allow(dead_code)]
    pub fn prev_page(&mut self) -> bool {
        if self.page_index > 0 {
            self.page_index -= 1;
            self.rerender();
            true
        } else {
            false
        }
    }

    /// Extract metadata for this portable document.
    pub fn extract_meta(&self, path: &Path) -> super::meta::DocumentMeta {
        let (width, height) = self.dimensions();
        #[allow(clippy::cast_possible_truncation)]
        super::meta::build_portable_meta(path, width, height, self.num_pages as u32)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Renderable for PortableDocument {
    fn render(&mut self, _scale: f64) -> DocResult<RenderOutput> {
        // PDF rendering quality is fixed for now (PDF_RENDER_QUALITY)
        let (width, height) = self.dimensions();
        Ok(RenderOutput {
            handle: self.handle.clone(),
            width,
            height,
        })
    }

    fn info(&self) -> DocumentInfo {
        let (width, height) = self.dimensions();
        DocumentInfo {
            width,
            height,
            format: "PDF".to_string(),
        }
    }
}

impl Transformable for PortableDocument {
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

impl MultiPage for PortableDocument {
    fn page_count(&self) -> usize {
        self.num_pages
    }

    fn current_page(&self) -> usize {
        self.page_index
    }

    fn go_to_page(&mut self, page: usize) -> DocResult<()> {
        if page >= self.num_pages {
            return Err(anyhow::anyhow!(
                "Page {} out of range (0-{})",
                page,
                self.num_pages - 1
            ));
        }
        self.page_index = page;
        self.rerender();
        Ok(())
    }
}

impl MultiPageThumbnails for PortableDocument {
    fn thumbnails_ready(&self) -> bool {
        self.thumbnail_cache
            .as_ref()
            .is_some_and(|c| c.len() >= self.num_pages)
    }

    fn thumbnails_loaded(&self) -> usize {
        PortableDocument::thumbnails_loaded(self)
    }

    fn generate_thumbnail_page(&mut self, page: usize) -> Option<usize> {
        PortableDocument::generate_thumbnail_page(self, page)
    }

    fn generate_all_thumbnails(&mut self) {
        if self.thumbnails_ready() {
            return;
        }
        self.init_thumbnail_cache();
        for page in 0..self.num_pages {
            self.generate_thumbnail_page(page);
        }
    }

    fn get_thumbnail(&self, page: usize) -> Option<ImageHandle> {
        self.thumbnail_cache
            .as_ref()
            .and_then(|cache| cache.get(page).cloned())
    }
}
