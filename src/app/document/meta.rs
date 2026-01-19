// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/document/meta.rs
//
// Document metadata extraction (basic info and EXIF).

use std::io::Cursor;
use std::path::Path;

use image::DynamicImage;
use exif::{In, Reader as ExifReader, Tag, Value};

use super::file;
use crate::constant::{MINUTES_PER_DEGREE, SECONDS_PER_DEGREE};

/// Basic document metadata (always available).
#[derive(Debug, Clone)]
pub struct BasicMeta {
    /// File name (without path).
    pub file_name: String,
    /// Full file path.
    pub file_path: String,
    /// Image format as string (e.g., "PNG", "JPEG", "PDF").
    pub format: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// File size in bytes.
    pub file_size: u64,
    /// Color type description (e.g., "RGBA8", "RGB8", "Grayscale").
    pub color_type: String,
}

impl BasicMeta {
    /// Format file size as human-readable string.
    pub fn file_size_display(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.file_size >= GB {
            format!("{:.2} GB", self.file_size as f64 / GB as f64)
        } else if self.file_size >= MB {
            format!("{:.2} MB", self.file_size as f64 / MB as f64)
        } else if self.file_size >= KB {
            format!("{:.1} KB", self.file_size as f64 / KB as f64)
        } else {
            format!("{} B", self.file_size)
        }
    }

    /// Format resolution as "W × H".
    pub fn resolution_display(&self) -> String {
        format!("{} × {}", self.width, self.height)
    }
}

/// EXIF metadata (optional, mainly for JPEG/TIFF).
#[derive(Debug, Clone, Default)]
pub struct ExifMeta {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub date_time: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<u32>,
    pub focal_length: Option<String>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
}

impl ExifMeta {
    /// Combined camera make and model for display.
    pub fn camera_display(&self) -> Option<String> {
        match (&self.camera_make, &self.camera_model) {
            (Some(make), Some(model)) => {
                if model.starts_with(make) {
                    Some(model.clone())
                } else {
                    Some(format!("{} {}", make, model))
                }
            }
            (Some(make), None) => Some(make.clone()),
            (None, Some(model)) => Some(model.clone()),
            (None, None) => None,
        }
    }

    /// Format GPS coordinates for display.
    pub fn gps_display(&self) -> Option<String> {
        match (self.gps_latitude, self.gps_longitude) {
            (Some(lat), Some(lon)) => Some(format!("{:.5}, {:.5}", lat, lon)),
            _ => None,
        }
    }
}

/// Complete document metadata container.
#[derive(Debug, Clone)]
pub struct DocumentMeta {
    pub basic: BasicMeta,
    pub exif: Option<ExifMeta>,
}

// ---------------------------------------------------------------------------
// Extraction functions
// ---------------------------------------------------------------------------

/// Extract basic metadata common to all document types.
fn extract_basic_meta(
    path: &Path,
    width: u32,
    height: u32,
    format: &str,
    color_type: String,
) -> BasicMeta {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let file_path = path.to_string_lossy().to_string();
    let file_size = file::file_size(path);

    BasicMeta {
        file_name,
        file_path,
        format: format.to_string(),
        width,
        height,
        file_size,
        color_type,
    }
}

/// Extract EXIF metadata from file bytes.
fn extract_exif_from_bytes(data: &[u8]) -> Option<ExifMeta> {
    let mut cursor = Cursor::new(data);
    let exif = ExifReader::new().read_from_container(&mut cursor).ok()?;

    let mut meta = ExifMeta::default();

    // Camera info.
    if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
        meta.camera_make = field.display_value().to_string().into();
    }
    if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
        meta.camera_model = field.display_value().to_string().into();
    }

    // Date/time.
    if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        meta.date_time = Some(field.display_value().to_string());
    } else if let Some(field) = exif.get_field(Tag::DateTime, In::PRIMARY) {
        meta.date_time = Some(field.display_value().to_string());
    }

    // Exposure settings.
    if let Some(field) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
        meta.exposure_time = Some(field.display_value().to_string());
    }
    if let Some(field) = exif.get_field(Tag::FNumber, In::PRIMARY) {
        meta.f_number = Some(format!("f/{}", field.display_value()));
    }
    if let Some(field) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY)
        && let Value::Short(ref vals) = field.value
            && let Some(&iso) = vals.first() {
                meta.iso = Some(iso as u32);
            }
    if let Some(field) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
        meta.focal_length = Some(field.display_value().to_string());
    }

    // GPS coordinates.
    meta.gps_latitude = extract_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef);
    meta.gps_longitude = extract_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef);

    Some(meta)
}

/// Extract a GPS coordinate (latitude or longitude) from EXIF data.
fn extract_gps_coord(exif: &exif::Exif, coord_tag: Tag, ref_tag: Tag) -> Option<f64> {
    let field = exif.get_field(coord_tag, In::PRIMARY)?;

    let degrees = match &field.value {
        Value::Rational(rats) if rats.len() >= 3 => {
            let d = rats[0].to_f64();
            let m = rats[1].to_f64();
            let s = rats[2].to_f64();
            d + m / MINUTES_PER_DEGREE + s / SECONDS_PER_DEGREE
        }
        _ => return None,
    };

    // Check reference (N/S or E/W) for sign.
    let sign = if let Some(ref_field) = exif.get_field(ref_tag, In::PRIMARY) {
        let ref_str = ref_field.display_value().to_string();
        if ref_str.contains('S') || ref_str.contains('W') {
            -1.0
        } else {
            1.0
        }
    } else {
        1.0
    };

    Some(degrees * sign)
}

/// Determine color type string from DynamicImage.
fn color_type_string(img: &DynamicImage) -> String {
    use image::DynamicImage::*;
    match img {
        ImageLuma8(_) => "Grayscale 8-bit".to_string(),
        ImageLumaA8(_) => "Grayscale+Alpha 8-bit".to_string(),
        ImageRgb8(_) => "RGB 8-bit".to_string(),
        ImageRgba8(_) => "RGBA 8-bit".to_string(),
        ImageLuma16(_) => "Grayscale 16-bit".to_string(),
        ImageLumaA16(_) => "Grayscale+Alpha 16-bit".to_string(),
        ImageRgb16(_) => "RGB 16-bit".to_string(),
        ImageRgba16(_) => "RGBA 16-bit".to_string(),
        ImageRgb32F(_) => "RGB 32-bit float".to_string(),
        ImageRgba32F(_) => "RGBA 32-bit float".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Determine format string from file extension.
fn format_from_extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_else(|| "Unknown".to_string())
}

// ---------------------------------------------------------------------------
// Public builder functions for each document type
// ---------------------------------------------------------------------------

/// Build metadata for a raster document.
pub fn build_raster_meta(path: &Path, img: &DynamicImage, width: u32, height: u32) -> DocumentMeta {
    let format = format_from_extension(path);
    let color_type = color_type_string(img);
    let basic = extract_basic_meta(path, width, height, &format, color_type);

    // Try to extract EXIF (mainly for JPEG/TIFF).
    let exif = file::read_file_bytes(path).and_then(|bytes| extract_exif_from_bytes(&bytes));

    DocumentMeta { basic, exif }
}

/// Build metadata for a vector document.
pub fn build_vector_meta(path: &Path, width: u32, height: u32) -> DocumentMeta {
    let basic = extract_basic_meta(path, width, height, "SVG", "Vector".to_string());

    DocumentMeta { basic, exif: None }
}

/// Build metadata for a portable document.
pub fn build_portable_meta(path: &Path, width: u32, height: u32, page_count: u32) -> DocumentMeta {
    let format = format!("PDF ({} pages)", page_count);
    let basic = extract_basic_meta(path, width, height, &format, "Rendered".to_string());

    DocumentMeta { basic, exif: None }
}
