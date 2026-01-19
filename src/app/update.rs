// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/update.rs
//
// Application update loop: applies messages to the global model state.

use cosmic::{Action, Task};

use super::document;
use super::message::AppMessage;
use super::model::{AppModel, ToolMode, ViewMode};
use crate::config::AppConfig;

// =============================================================================
// Update Result
// =============================================================================

pub enum UpdateResult {
    None,
    Task(Task<Action<AppMessage>>),
}

// =============================================================================
// Main Update Function
// =============================================================================

pub fn update(model: &mut AppModel, msg: &AppMessage, config: &AppConfig) -> UpdateResult {
    match msg {
        // ---- File / navigation ----------------------------------------------------
        AppMessage::OpenPath(path) => {
            document::file::open_single_file(model, path);
        }

        AppMessage::NextDocument => {
            document::file::navigate_next(model);
        }

        AppMessage::PrevDocument => {
            document::file::navigate_prev(model);
        }

        AppMessage::GotoPage(page) => {
            if let Some(doc) = &mut model.document
                && let Err(e) = doc.go_to_page(*page) {
                    log::error!("Failed to navigate to page {}: {}", page, e);
                }
        }

        // ---- Thumbnail generation -------------------------------------------------
        AppMessage::GenerateThumbnailPage(page) => {
            if let Some(doc) = &mut model.document
                && let Some(next_page) = doc.generate_thumbnail_page(*page) {
                    return UpdateResult::Task(Task::batch([
                        Task::future(async move {
                            Action::App(AppMessage::GenerateThumbnailPage(next_page))
                        }),
                        Task::done(Action::App(AppMessage::RefreshView)),
                    ]));
                }
        }

        AppMessage::RefreshView => {
            model.tick += 1;
        }

        // ---- View / zoom ---------------------------------------------------------
        AppMessage::ZoomIn => {
            zoom_in(model, config);
        }

        AppMessage::ZoomOut => {
            zoom_out(model, config);
        }

        AppMessage::ZoomReset => {
            model.view_mode = ViewMode::ActualSize;
            model.reset_pan();
        }

        AppMessage::ZoomFit => {
            model.view_mode = ViewMode::Fit;
            model.reset_pan();
        }

        AppMessage::ViewerStateChanged {
            scale,
            offset_x,
            offset_y,
        } => {
            model.view_mode = ViewMode::Custom(*scale);
            model.pan_x = *offset_x;
            model.pan_y = *offset_y;
        }

        // ---- Pan control ---------------------------------------------------------
        AppMessage::PanLeft => {
            model.pan_x -= config.pan_step;
        }
        AppMessage::PanRight => {
            model.pan_x += config.pan_step;
        }
        AppMessage::PanUp => {
            model.pan_y -= config.pan_step;
        }
        AppMessage::PanDown => {
            model.pan_y += config.pan_step;
        }
        AppMessage::PanReset => {
            model.reset_pan();
        }

        // ---- Tool modes ----------------------------------------------------------
        AppMessage::ToggleCropMode => {
            model.tool_mode = if model.tool_mode == ToolMode::Crop {
                ToolMode::None
            } else {
                ToolMode::Crop
            };
        }
        AppMessage::ToggleScaleMode => {
            model.tool_mode = if model.tool_mode == ToolMode::Scale {
                ToolMode::None
            } else {
                ToolMode::Scale
            };
        }

        // ---- Document transformations --------------------------------------------
        AppMessage::FlipHorizontal => {
            if let Some(doc) = &mut model.document {
                doc.flip_horizontal();
            }
        }
        AppMessage::FlipVertical => {
            if let Some(doc) = &mut model.document {
                doc.flip_vertical();
            }
        }
        AppMessage::RotateCW => {
            if let Some(doc) = &mut model.document {
                doc.rotate_cw();
            }
        }
        AppMessage::RotateCCW => {
            if let Some(doc) = &mut model.document {
                doc.rotate_ccw();
            }
        }

        // ---- Metadata ------------------------------------------------------------
        AppMessage::RefreshMetadata => {
            refresh_metadata(model);
        }

        // ---- Wallpaper -----------------------------------------------------------
        AppMessage::SetAsWallpaper => {
            set_as_wallpaper(model);
        }

        // ---- Error handling ------------------------------------------------------
        AppMessage::ShowError(msg) => {
            model.set_error(msg.clone());
        }
        AppMessage::ClearError => {
            model.clear_error();
        }

        // ---- Handled elsewhere ---------------------------------------------------
        AppMessage::ToggleContextPage(_) | AppMessage::ToggleNavBar => {}

        AppMessage::NoOp => {}
    }

    UpdateResult::None
}

// =============================================================================
// View Helpers
// =============================================================================

fn zoom_in(model: &mut AppModel, config: &AppConfig) {
    let current = current_zoom(model);
    let new_zoom = (current * config.scale_step).clamp(config.min_scale, config.max_scale);
    let factor = new_zoom / current;
    model.pan_x *= factor;
    model.pan_y *= factor;
    model.view_mode = ViewMode::Custom(new_zoom);
}

fn zoom_out(model: &mut AppModel, config: &AppConfig) {
    let current = current_zoom(model);
    let new_zoom = (current / config.scale_step).clamp(config.min_scale, config.max_scale);
    let factor = new_zoom / current;
    model.pan_x *= factor;
    model.pan_y *= factor;
    model.view_mode = ViewMode::Custom(new_zoom);
}

fn current_zoom(model: &AppModel) -> f32 {
    match model.view_mode {
        ViewMode::Fit | ViewMode::ActualSize => 1.0,
        ViewMode::Custom(z) => z,
    }
}

fn refresh_metadata(model: &mut AppModel) {
    model.metadata = match (&model.document, &model.current_path) {
        (Some(doc), Some(path)) => Some(doc.extract_meta(path)),
        _ => None,
    };
}

fn set_as_wallpaper(model: &mut AppModel) {
    let Some(path) = model.current_path.as_ref() else {
        model.set_error("No image loaded");
        return;
    };
    document::set_as_wallpaper(path);
}
