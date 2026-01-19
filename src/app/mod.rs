// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/mod.rs
//
// Application module root, re-exports, and COSMIC application wiring.

pub mod document;
pub mod message;
pub mod model;
pub mod update;

mod view;

use std::time::Duration;

use cosmic::app::{context_drawer, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::keyboard::{self, key::Named, Key, Modifiers};
use cosmic::iced::time;
use cosmic::iced::window;
use cosmic::iced::Subscription;
use cosmic::widget::nav_bar;
use cosmic::{Action, Element, Task};

pub use message::AppMessage;
pub use model::AppModel;

use crate::config::AppConfig;
use crate::Args;

/// Flags passed from `main` into the application.
#[derive(Debug, Clone)]
pub enum Flags {
    Args(Args),
}

/// Context page displayed in right drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    Properties,
}

/// Main application type.
pub struct Noctua {
    core: Core,
    pub model: AppModel,
    nav: nav_bar::Model,
    context_page: ContextPage,
    config: AppConfig,
    config_handler: Option<cosmic_config::Config>,
}

impl cosmic::Application for Noctua {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = Flags;
    type Message = AppMessage;

    const APP_ID: &'static str = "org.codeberg.wfx.Noctua";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, Task<Action<Self::Message>>) {
        // Load persisted config.
        let (config, config_handler) =
            match cosmic_config::Config::new(Self::APP_ID, AppConfig::VERSION) {
                Ok(handler) => {
                    let config = AppConfig::get_entry(&handler).unwrap_or_default();
                    (config, Some(handler))
                }
                Err(_) => (AppConfig::default(), None),
            };

        let mut model = AppModel::new(config.clone());

        let Flags::Args(args) = flags;

        // Determine initial path: CLI argument takes priority.
        // Fall back to configured default directory only if it exists.
        let initial_path = args.file.or_else(|| {
            config
                .default_image_dir
                .as_ref()
                .filter(|p| p.exists())
                .cloned()
        });

        if let Some(path) = initial_path {
            document::file::open_initial_path(&mut model, path);
        }

        // Initialize nav bar model (required for COSMIC to show toggle icon).
        let nav = nav_bar::Model::default();

        // Apply persisted panel states.
        core.window.show_context = config.context_drawer_visible;
        core.nav_bar_set_toggled(config.nav_bar_visible);

        // Start thumbnail generation for initial document if applicable.
        let init_task = start_thumbnail_generation(&model);

        (
            Self {
                core,
                model,
                nav,
                context_page: ContextPage::default(),
                config,
                config_handler,
            },
            init_task,
        )
    }

    fn on_close_requested(&self, _id: window::Id) -> Option<Self::Message> {
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Action<Self::Message>> {
        match &message {
            AppMessage::ToggleNavBar => {
                self.core.nav_bar_toggle();
                let is_visible = self.core.nav_bar_active();
                self.config.nav_bar_visible = is_visible;
                self.save_config();

                if is_visible {
                    return start_thumbnail_generation_task(&self.model);
                }
                return Task::none();
            }

            AppMessage::ToggleContextPage(page) => {
                if self.context_page == *page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = *page;
                    self.core.window.show_context = true;
                }
                self.config.context_drawer_visible = self.core.window.show_context;
                self.save_config();
                return Task::none();
            }

            AppMessage::OpenPath(_) | AppMessage::NextDocument | AppMessage::PrevDocument => {
                let result = update::update(&mut self.model, &message, &self.config);
                let thumb_task = start_thumbnail_generation_task(&self.model);
                return match result {
                    update::UpdateResult::None => thumb_task,
                    update::UpdateResult::Task(task) => Task::batch([task, thumb_task]),
                };
            }

            _ => {}
        }

        match update::update(&mut self.model, &message, &self.config) {
            update::UpdateResult::None => Task::none(),
            update::UpdateResult::Task(task) => task,
        }
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        view::header::start(&self.model)
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        view::header::end(&self.model)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        view::view(&self.model, &self.config)
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }
        Some(context_drawer::context_drawer(
            view::panels::view(&self.model),
            AppMessage::ToggleContextPage(ContextPage::Properties),
        ))
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn nav_bar(&self) -> Option<Element<'_, Action<Self::Message>>> {
        if !self.core.nav_bar_active() {
            return None;
        }
        view::nav_bar(&self.model)
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        Some(view::footer::view(&self.model))
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            keyboard::on_key_press(handle_key_press),
            thumbnail_refresh_subscription(self),
        ])
    }
}

impl Noctua {
    /// Save current config to disk.
    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            let _ = self.config.write_entry(handler);
        }
    }
}

/// Map raw key presses + modifiers into high-level application messages.
fn handle_key_press(key: Key, modifiers: Modifiers) -> Option<AppMessage> {
    use AppMessage::*;

    // Handle Ctrl + arrow keys for panning.
    if modifiers.control() && !modifiers.shift() && !modifiers.alt() && !modifiers.logo() {
        return match key.as_ref() {
            Key::Named(Named::ArrowLeft) => Some(PanLeft),
            Key::Named(Named::ArrowRight) => Some(PanRight),
            Key::Named(Named::ArrowUp) => Some(PanUp),
            Key::Named(Named::ArrowDown) => Some(PanDown),
            _ => None,
        };
    }

    // Ignore key presses when command-style modifiers are pressed.
    if modifiers.command() || modifiers.alt() || modifiers.logo() || modifiers.control() {
        return None;
    }

    match key.as_ref() {
        // Navigation with arrow keys (no modifiers).
        Key::Named(Named::ArrowRight) => Some(NextDocument),
        Key::Named(Named::ArrowLeft) => Some(PrevDocument),

        // Transformations.
        Key::Character(ch) if ch.eq_ignore_ascii_case("h") => Some(FlipHorizontal),
        Key::Character(ch) if ch.eq_ignore_ascii_case("v") => Some(FlipVertical),
        Key::Character(ch) if ch.eq_ignore_ascii_case("r") => {
            if modifiers.shift() {
                Some(RotateCCW)
            } else {
                Some(RotateCW)
            }
        }

        // Zoom.
        Key::Character("+" | "=") => Some(ZoomIn),
        Key::Character("-") => Some(ZoomOut),
        Key::Character("1") => Some(ZoomReset),
        Key::Character(ch) if ch.eq_ignore_ascii_case("f") => Some(ZoomFit),

        // Tool modes.
        Key::Character(ch) if ch.eq_ignore_ascii_case("c") => Some(ToggleCropMode),
        Key::Character(ch) if ch.eq_ignore_ascii_case("s") => Some(ToggleScaleMode),

        // Reset pan.
        Key::Character("0") => Some(PanReset),

        // Toggle panels.
        Key::Character(ch) if ch.eq_ignore_ascii_case("i") => {
            Some(ToggleContextPage(ContextPage::Properties))
        }
        Key::Character(ch) if ch.eq_ignore_ascii_case("n") => Some(ToggleNavBar),

        // Wallpaper.
        Key::Character(ch) if ch.eq_ignore_ascii_case("w") => Some(SetAsWallpaper),

        _ => None,
    }
}

// =============================================================================
// Thumbnail Helpers
// =============================================================================

fn start_thumbnail_generation(model: &AppModel) -> Task<Action<AppMessage>> {
    start_thumbnail_generation_task(model)
}

fn start_thumbnail_generation_task(model: &AppModel) -> Task<Action<AppMessage>> {
    if let Some(doc) = &model.document {
        let page_count = doc.page_count().unwrap_or(0);
        if page_count > 0 && !doc.thumbnails_ready() {
            return Task::batch([
                Task::done(Action::App(AppMessage::GenerateThumbnailPage(0))),
                Task::done(Action::App(AppMessage::RefreshView)),
            ]);
        }
    }
    Task::none()
}

fn thumbnail_refresh_subscription(app: &Noctua) -> Subscription<AppMessage> {
    let needs_refresh = app
        .model
        .document
        .as_ref()
        .is_some_and(|doc| doc.is_multi_page() && !doc.thumbnails_ready());

    if needs_refresh {
        time::every(Duration::from_millis(100)).map(|_| AppMessage::RefreshView)
    } else {
        Subscription::none()
    }
}
