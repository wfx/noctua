// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/header.rs
//
// Header bar content (navigation, rotation, flip).

use cosmic::iced::Length;
use cosmic::widget::{button, horizontal_space, icon, row};
use cosmic::Element;

use crate::app::message::AppMessage;
use crate::app::model::AppModel;
use crate::app::ContextPage;

/// Build the start (left) side of the header bar.
pub fn start(model: &AppModel) -> Vec<Element<'_, AppMessage>> {
    let has_doc = model.document.is_some();

    // Left: Nav toggle + Navigation
    let left_controls = row()
        .push(
            button::icon(icon::from_name("go-previous-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::PrevDocument)),
        )
        .push(
            button::icon(icon::from_name("go-next-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::NextDocument)),
        );

    // Center: Transformations (horizontally centered)
    let center_controls = row()
        //.align_y(Alignment::Center)
        .push(
            button::icon(icon::from_name("object-rotate-left-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::RotateCCW)),
        )
        .push(
            button::icon(icon::from_name("object-rotate-right-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::RotateCW)),
        )
        .push(horizontal_space().width(Length::Fixed(12.0)))
        .push(
            button::icon(icon::from_name("object-flip-horizontal-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::FlipHorizontal)),
        )
        .push(
            button::icon(icon::from_name("object-flip-vertical-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::FlipVertical)),
        );

    vec![
        left_controls.into(),
        //horizontal_space().width(Length::Fill).into(),
        center_controls.into(),
        horizontal_space().width(Length::Fill).into(),
    ]
}

/// Build the end (right) side of the header bar.
pub fn end(_model: &AppModel) -> Vec<Element<'_, AppMessage>> {
    vec![
        // Info panel toggle
        button::icon(icon::from_name("dialog-information-symbolic"))
            .on_press(AppMessage::ToggleContextPage(ContextPage::Properties))
            .into(),
    ]
}
