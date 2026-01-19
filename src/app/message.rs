// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/message.rs
//
// Application messages: events, user actions, and internal signals.

use std::path::PathBuf;

use crate::app::ContextPage;

#[derive(Debug, Clone)]
pub enum AppMessage {
    // File / navigation.
    #[allow(dead_code)]
    OpenPath(PathBuf),
    NextDocument,
    PrevDocument,
    GotoPage(usize),
    GenerateThumbnailPage(usize),

    // Transformations.
    RotateCW,
    RotateCCW,
    FlipHorizontal,
    FlipVertical,

    // View / zoom.
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomFit,
    ViewerStateChanged {
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    },

    // Pan control.
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    PanReset,

    // Tool modes.
    ToggleCropMode,
    ToggleScaleMode,

    // Panels.
    ToggleContextPage(ContextPage),
    ToggleNavBar,

    // Metadata.
    #[allow(dead_code)]
    RefreshMetadata,

    // Wallpaper.
    SetAsWallpaper,

    // Errors.
    #[allow(dead_code)]
    ShowError(String),
    #[allow(dead_code)]
    ClearError,

    // UI refresh.
    RefreshView,

    // Fallback.
    #[allow(dead_code)]
    NoOp,
}
