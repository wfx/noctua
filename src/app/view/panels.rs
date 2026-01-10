// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/panels.rs
//
// Header, footer, and side panels composing the main layout.

use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, Column, Container, Row, Text};

use crate::fl;
use crate::app::model::ViewMode;
use crate::app::{AppMessage, AppModel};

/// Top header bar (global actions, toggles).
pub fn header(model: &AppModel) -> Element<'_, AppMessage> {
    // Left panel toggle button.
    let left_toggle = widget::button::icon(widget::icon::from_name(if model.show_left_panel {
        "sidebar-show-left-symbolic"
    } else {
        "sidebar-show-left-symbolic"
    }))
    .on_press(AppMessage::ToggleLeftPanel);

    // Right panel toggle button.
    let right_toggle = widget::button::icon(widget::icon::from_name(if model.show_right_panel {
        "sidebar-show-right-symbolic"
    } else {
        "sidebar-show-right-symbolic"
    }))
    .on_press(AppMessage::ToggleRightPanel);

    // File name display (centered).
    let file_name = model
        .current_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let title = Text::new(file_name);

    // Spacer to push title to center and right_toggle to the right.
    let left_section = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(left_toggle);

    let center_section = Container::new(title)
        .width(Length::Fill)
        .align_x(Alignment::Center);

    let right_section = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(right_toggle);

    let content = Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(left_section)
        .push(center_section)
        .push(right_section);

    Container::new(content)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
}

/// Bottom footer bar (navigation & zoom).
pub fn footer(model: &AppModel) -> Element<'_, AppMessage> {
    let nav = Row::new()
        .spacing(4)
        .align_y(Alignment::Center)
        .push(widget::button::standard("<").on_press(AppMessage::PrevDocument))
        .push(widget::button::standard(">").on_press(AppMessage::NextDocument));

    let zoom_text = match model.view_mode {
        ViewMode::Fit => "Fit".to_string(),
        ViewMode::ActualSize => "100%".to_string(),
        ViewMode::Custom(zoom_factor) => format!("{:.0}%", zoom_factor * 100.0),
    };

    let zoom_info = Text::new(format!("Zoom: {}", zoom_text));

    let content = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(nav)
        .push(zoom_info);

    Container::new(content)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
}

/// Optional left panel (tools).
pub fn left_panel(model: &AppModel) -> Option<Element<'_, AppMessage>> {
    if !model.show_left_panel {
        return None;
    }

    let tools = Column::new()
        .spacing(4)
        .push(Text::new(fl!("tools")))
        .push(widget::button::standard(fl!("crop")).on_press(AppMessage::ToggleCropMode))
        .push(widget::button::standard(fl!("scale")).on_press(AppMessage::ToggleScaleMode));

    let panel = Container::new(tools)
        .width(Length::Fixed(180.0))
        .height(Length::Fill)
        .padding(8);

    Some(panel.into())
}

/// Optional right panel (metadata, info).
pub fn right_panel(model: &AppModel) -> Option<Element<'_, AppMessage>> {
    if !model.show_right_panel {
        return None;
    }

    let mut content = Column::new().spacing(8).padding(4);

    // Section header.
    content = content.push(Text::new(fl!("metadata")).size(16).width(Length::Fill));

    content = content.push(widget::divider::horizontal::default());

    if let Some(meta) = &model.metadata {
        // Basic information section.
        content = content
            .push(meta_row(fl!("file-name"), meta.basic.file_name.clone()))
            .push(meta_row(fl!("format"), meta.basic.format.clone()))
            .push(meta_row(fl!("resolution"), meta.basic.resolution_display()))
            .push(meta_row(fl!("file-size"), meta.basic.file_size_display()))
            .push(meta_row(fl!("color-type"), meta.basic.color_type.clone()));

        // EXIF section (if available).
        if let Some(exif) = &meta.exif {
            content = content
                .push(widget::vertical_space().height(Length::Fixed(12.0)))
                .push(Text::new(fl!("exif-data")).size(14))
                .push(widget::divider::horizontal::default());

            if let Some(camera) = exif.camera_display() {
                content = content.push(meta_row(fl!("camera"), camera));
            }
            if let Some(date) = &exif.date_time {
                content = content.push(meta_row(fl!("date-taken"), date.clone()));
            }
            if let Some(exp) = &exif.exposure_time {
                content = content.push(meta_row(fl!("exposure"), exp.clone()));
            }
            if let Some(aperture) = &exif.f_number {
                content = content.push(meta_row(fl!("aperture"), aperture.clone()));
            }
            if let Some(iso) = exif.iso {
                content = content.push(meta_row(fl!("iso"), iso.to_string()));
            }
            if let Some(focal) = &exif.focal_length {
                content = content.push(meta_row(fl!("focal-length"), focal.clone()));
            }
            if let Some(gps) = exif.gps_display() {
                content = content.push(meta_row(fl!("gps"), gps));
            }
        }
    } else if model.document.is_some() {
        // Document exists but metadata not yet loaded.
        content = content.push(Text::new(fl!("loading-metadata")));
    } else {
        // No document loaded.
        content = content.push(Text::new(fl!("no-document")));
    }

    let panel = Container::new(widget::scrollable(content).height(Length::Fill))
        .width(Length::Fixed(240.0))
        .height(Length::Fill)
        .padding(8);

    Some(panel.into())
}

/// Helper to create a label-value row for metadata display.
fn meta_row(label: String, value: String) -> Element<'static, AppMessage> {
    Row::new()
        .spacing(8)
        .push(
            Text::new(format!("{}:", label))
                .size(12)
                .width(Length::Fixed(80.0)),
        )
        .push(Text::new(value).size(12).width(Length::Fill))
        .into()
}
