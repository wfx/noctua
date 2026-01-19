// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/pages_panel.rs
//
// Page navigation panel for multi-page documents (PDF, multi-page TIFF, etc.).

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, scrollable, text};
use cosmic::widget::image as cosmic_image;
use cosmic::Element;

use crate::app::{AppMessage, AppModel};
use crate::constant::THUMBNAIL_MAX_WIDTH;
use crate::fl;

/// Build the page navigation panel view.
/// Returns None if the current document doesn't support multiple pages.
pub fn view(model: &AppModel) -> Option<Element<'static, AppMessage>> {
    let doc = model.document.as_ref()?;

    // Only show for multi-page documents.
    if !doc.is_multi_page() {
        return None;
    }

    let page_count = doc.page_count()?;
    let loaded = doc.thumbnails_loaded();
    let current_page = doc.current_page()?;

    let mut content = column::with_capacity(page_count + 1)
        .spacing(12)
        .padding([12, 8])
        .align_x(Alignment::Center)
        .width(Length::Fill);

    // Show loading progress if not all thumbnails are ready.
    if !doc.thumbnails_ready() {
        let loading_msg = fl!("loading-thumbnails", current: loaded, total: page_count);
        content = content.push(text::caption(loading_msg));
    }

    // Build thumbnail list for pages that are already loaded.
    for page_index in 0..loaded {
        let is_current = page_index == current_page;

        // Get cached thumbnail handle.
        let thumbnail_element: Element<'static, AppMessage> =
            if let Some(handle) = doc.get_thumbnail(page_index) {
                cosmic_image::Image::new(handle)
                    .width(Length::Fixed(THUMBNAIL_MAX_WIDTH))
                    .into()
            } else {
                // Fallback: show page number if no thumbnail.
                text::body(format!("{}", page_index + 1)).into()
            };

        // Page number label.
        let page_label = text::caption(format!("{}", page_index + 1));

        // Combine thumbnail and label in a column.
        let page_content = column::with_capacity(2)
            .spacing(4)
            .align_x(Alignment::Center)
            .push(thumbnail_element)
            .push(page_label);

        // Wrap in button for navigation.
        let page_button = if is_current {
            // Current page: highlighted style.
            button::custom(page_content)
                .class(cosmic::theme::Button::Suggested)
                .padding(4)
        } else {
            // Other pages: clickable with standard style.
            button::custom(page_content)
                .class(cosmic::theme::Button::Standard)
                .padding(4)
                .on_press(AppMessage::GotoPage(page_index))
        };

        content = content.push(page_button);
    }

    // Wrap in scrollable container.
    Some(
        scrollable(content)
            .width(Length::Shrink)
            .height(Length::Fill)
            .into(),
    )
}
