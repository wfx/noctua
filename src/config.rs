// SPDX-License-Identifier: GPL-3.0-or-later
// src/config.rs
//
// Global configuration for the application with cosmic-config support.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use std::path::PathBuf;

/// Global configuration for the application.
#[derive(Debug, Clone, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct AppConfig {
    /// Default directory to open when browsing for documents.
    pub default_image_dir: Option<PathBuf>,
    /// Show page navigation panel (left sidebar for multi-page documents).
    pub nav_bar_visible: bool,
    /// Show properties panel (right sidebar with metadata).
    pub context_drawer_visible: bool,
    /// Zoom step multiplier for keyboard shortcuts (1.1 = 10% increase per step).
    pub scale_step: f32,
    /// Pan distance in pixels per arrow key press.
    pub pan_step: f32,
    /// Minimum zoom level (0.1 = 10% of original size).
    pub min_scale: f32,
    /// Maximum zoom level (8.0 = 800% of original size).
    pub max_scale: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_image_dir: dirs::picture_dir().or_else(dirs::home_dir),
            nav_bar_visible: false,
            context_drawer_visible: false,
            scale_step: 1.1,
            pan_step: 50.0,
            min_scale: 0.1,
            max_scale: 8.0,
        }
    }
}
