// SPDX-License-Identifier: GPL-3.0-or-later
// src/constant.rs
//
// Application constants that should not be changed by the user.

/// Minutes per degree (GPS coordinate conversion: DMS to decimal degrees).
pub const MINUTES_PER_DEGREE: f64 = 60.0;

/// Seconds per degree (GPS coordinate conversion: DMS to decimal degrees).
pub const SECONDS_PER_DEGREE: f64 = 3600.0;

/// Minimum pixmap size for SVG rendering (prevents zero-size pixmaps).
pub const MIN_PIXMAP_SIZE: u32 = 1;

/// Tolerance for scale comparisons (float precision in zoom synchronization).
pub const SCALE_EPSILON: f32 = 0.0001;

/// Tolerance for offset comparisons (float precision in pan synchronization).
pub const OFFSET_EPSILON: f32 = 0.01;

/// Maximum width in pixels for page navigation thumbnails.
pub const THUMBNAIL_MAX_WIDTH: f32 = 100.0;

/// Cache directory name under ~/.cache/ for thumbnail storage.
pub const CACHE_DIR: &str = "noctua";

/// File extension for cached thumbnails.
pub const THUMBNAIL_EXT: &str = "png";

/// PDF page render quality multiplier (2.0 = double resolution for sharp display).
pub const PDF_RENDER_QUALITY: f64 = 2.0;

/// PDF thumbnail size multiplier (0.25 = 25% for fast preview generation).
pub const PDF_THUMBNAIL_SIZE: f64 = 0.25;
