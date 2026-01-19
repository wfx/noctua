// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/mod.rs
//
// View module root, combining all view components.

mod canvas;
pub mod footer;
pub mod header;
mod image_viewer;
pub mod pages_panel;
pub mod panels;

use cosmic::iced::Length;
use cosmic::widget::container;
use cosmic::{Action, Element};

use crate::app::{AppMessage, AppModel};
use crate::config::AppConfig;

/// Main application view (canvas area).
pub fn view<'a>(model: &'a AppModel, config: &'a AppConfig) -> Element<'a, AppMessage> {
    canvas::view(model, config)
}

/// Navigation bar content (left panel for multi-page documents).
///
/// Returns None if no multi-page document is loaded.
pub fn nav_bar(model: &AppModel) -> Option<Element<'_, Action<AppMessage>>> {
    let doc = model.document.as_ref()?;
    if !doc.is_multi_page() {
        return None;
    }

    pages_panel::view(model).map(|panel| {
        container(panel.map(Action::App))
            .width(Length::Shrink)
            .height(Length::Fill)
            .max_width(200)
            .into()
    })
}
