//! Main application state and logic

use crate::discovery::AppEntry;
use crate::ipc::IpcCommand;
use crate::search::fuzzy_search;
use crate::ui::theme::{self, WINDOW_HEIGHT, WINDOW_WIDTH};

use iced::widget::{
    button, column, container, image, row, scrollable, text, text_input, Column, Image,
};
use iced::{
    border::Radius,
    keyboard, window, Background, Border, Color, ContentFit, Element, Event, Fill,
    Subscription, Task, Theme,
};

/// Pinned apps that always appear at the top (case-insensitive partial match)
const PINNED_APPS: &[&str] = &[
    "Visual Studio Code",
    "Visual Studio 2022",
    "Microsoft Teams",
    "Outlook",
    "Microsoft Excel",
    "Visio",
    "PowerPoint",
];

/// Main application state
pub struct Launcher {
    /// Current search query
    search_query: String,
    /// All discovered applications
    all_apps: Vec<AppEntry>,
    /// Filtered apps based on search
    filtered_apps: Vec<AppEntry>,
    /// Currently selected app index (for keyboard navigation)
    selected_index: usize,
    /// Whether window is currently visible
    is_visible: bool,
    /// Colors derived from the GlazeWM config (read once at startup)
    colors: theme::Colors,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Search input changed
    SearchChanged(String),
    /// App was clicked to launch
    LaunchApp(usize),
    /// Apps were discovered
    AppsLoaded(Vec<AppEntry>),
    /// Keyboard event
    KeyPressed(keyboard::Key),
    /// IPC command received
    IpcCommand(IpcCommand),
    /// Focus search input
    FocusSearch,
    /// Window lost focus - close launcher
    WindowUnfocused,
}

impl Launcher {
    /// Initialize the launcher with colors derived from the GlazeWM config.
    /// `visible` is false when started hidden in the tray.
    pub fn new(colors: theme::Colors, visible: bool) -> (Self, Task<Message>) {
        // Load cached apps or discover them
        let apps = crate::discovery::load_cached_apps().unwrap_or_default();
        let sorted_apps = Self::sort_with_pinned(apps);

        let launcher = Self {
            search_query: String::new(),
            filtered_apps: sorted_apps.clone(),
            all_apps: sorted_apps,
            selected_index: 0,
            is_visible: visible,
            colors,
        };

        // Start async app discovery to refresh cache
        let task = Task::perform(
            async { crate::discovery::discover_apps().await },
            Message::AppsLoaded,
        );

        // Chain with focus task
        let focus_task = text_input::focus(text_input::Id::new("search"));

        (launcher, Task::batch([task, focus_task]))
    }

    /// Sort apps with pinned apps at the top
    fn sort_with_pinned(mut apps: Vec<AppEntry>) -> Vec<AppEntry> {
        // Separate pinned and regular apps
        let mut pinned = Vec::new();
        let mut regular = Vec::new();

        for app in apps.drain(..) {
            let app_name_lower = app.name.to_lowercase();
            let is_pinned = PINNED_APPS.iter().any(|p| {
                app_name_lower.contains(&p.to_lowercase())
            });

            if is_pinned {
                pinned.push(app);
            } else {
                regular.push(app);
            }
        }

        // Sort pinned apps by their order in PINNED_APPS
        pinned.sort_by(|a, b| {
            let a_idx = PINNED_APPS.iter().position(|p| a.name.to_lowercase().contains(&p.to_lowercase())).unwrap_or(999);
            let b_idx = PINNED_APPS.iter().position(|p| b.name.to_lowercase().contains(&p.to_lowercase())).unwrap_or(999);
            a_idx.cmp(&b_idx)
        });

        // Regular apps already sorted alphabetically
        pinned.extend(regular);
        pinned
    }

    /// Handle messages
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query.clone();
                self.selected_index = 0; // Reset selection on search change
                if query.is_empty() {
                    self.filtered_apps = self.all_apps.clone();
                } else {
                    self.filtered_apps = fuzzy_search(&self.all_apps, &query);
                }
                // Scroll to top when searching
                scrollable::scroll_to(
                    scrollable::Id::new("app_list"),
                    scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                )
            }

            Message::WindowUnfocused => {
                // Close launcher when clicking outside
                self.search_query.clear();
                self.filtered_apps = self.all_apps.clone();
                self.selected_index = 0;
                self.is_visible = false;
                window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Hidden))
            }

            Message::LaunchApp(index) => {
                if let Some(app) = self.filtered_apps.get(index) {
                    if let Some(ref path) = app.exec_path {
                        launch_path(path);
                    }
                }
                // Hide after launch
                self.search_query.clear();
                self.filtered_apps = self.all_apps.clone();
                self.is_visible = false;
                window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Hidden))
            }

            Message::AppsLoaded(apps) => {
                let sorted_apps = Self::sort_with_pinned(apps);
                self.all_apps = sorted_apps.clone();
                if self.search_query.is_empty() {
                    self.filtered_apps = sorted_apps;
                } else {
                    self.filtered_apps = fuzzy_search(&self.all_apps, &self.search_query);
                }
                self.selected_index = 0;
                // Cache apps for next launch
                let apps_to_cache = self.all_apps.clone();
                Task::perform(
                    async move {
                        let _ = crate::discovery::cache_apps(&apps_to_cache).await;
                        apps_to_cache
                    },
                    |_| Message::FocusSearch,
                )
            }

            Message::KeyPressed(key) => {
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    self.search_query.clear();
                    self.filtered_apps = self.all_apps.clone();
                    self.selected_index = 0;
                    self.is_visible = false;
                    return window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Hidden));
                }
                if key == keyboard::Key::Named(keyboard::key::Named::ArrowDown) {
                    if !self.filtered_apps.is_empty() {
                        self.selected_index = (self.selected_index + 1) % self.filtered_apps.len();
                    }
                    return self.scroll_to_selected();
                }
                if key == keyboard::Key::Named(keyboard::key::Named::ArrowUp) {
                    if !self.filtered_apps.is_empty() {
                        if self.selected_index == 0 {
                            self.selected_index = self.filtered_apps.len() - 1;
                        } else {
                            self.selected_index -= 1;
                        }
                    }
                    return self.scroll_to_selected();
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Enter) {
                    // Launch selected result
                    if !self.filtered_apps.is_empty() {
                        if let Some(ref path) = self.filtered_apps[self.selected_index].exec_path {
                            launch_path(path);
                        }
                        self.search_query.clear();
                        self.filtered_apps = self.all_apps.clone();
                        self.selected_index = 0;
                        self.is_visible = false;
                        return window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Hidden));
                    }
                }
                Task::none()
            }

            Message::FocusSearch => text_input::focus(text_input::Id::new("search")),

            Message::IpcCommand(cmd) => {
                match cmd {
                    IpcCommand::Toggle => {
                        if self.is_visible {
                            self.hide_window()
                        } else {
                            self.show_window()
                        }
                    }
                    IpcCommand::Show => self.show_window(),
                    IpcCommand::Hide => self.hide_window(),
                    IpcCommand::Quit => iced::exit(),
                }
            }
        }
    }

    /// Show the launcher window
    fn show_window(&mut self) -> Task<Message> {
        self.is_visible = true;
        self.search_query.clear();
        self.filtered_apps = self.all_apps.clone();
        self.selected_index = 0;

        // Unhide and focus window
        Task::batch([
            window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Windowed)),
            window::get_latest().and_then(window::gain_focus),
            text_input::focus(text_input::Id::new("search")),
        ])
    }

    /// Hide the launcher window
    fn hide_window(&mut self) -> Task<Message> {
        self.is_visible = false;
        window::get_latest().and_then(|id| window::change_mode(id, window::Mode::Hidden))
    }

    /// Build the view: search bar above the app list
    pub fn view(&self) -> Element<'_, Message> {
        // Search bar
        let search = text_input("Search...", &self.search_query)
            .id(text_input::Id::new("search"))
            .on_input(Message::SearchChanged)
            .padding([12, 18])
            .size(theme::SEARCH_FONT_SIZE)
            .width(Fill)
            .style(|_theme, _status| text_input::Style {
                background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.92)),
                border: Border {
                    radius: Radius::from(theme::SEARCH_RADIUS),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                icon: Color::from_rgb(0.4, 0.4, 0.4),
                placeholder: Color::from_rgb(0.5, 0.5, 0.5),
                value: Color::from_rgb(0.2, 0.2, 0.2),
                selection: Color::from_rgba(0.3, 0.5, 0.8, 0.3),
            });

        // App list
        let app_list = self.view_app_list();

        // Combine into column
        let content = column![search, app_list]
            .spacing(theme::APP_LIST_TOP_MARGIN)
            .padding(theme::PANEL_PADDING);

        // Wrap in a container with rounded corners and border
        // clip(true) ensures content is clipped to rounded corners
        let colors = self.colors;
        container(content)
            .width(WINDOW_WIDTH)
            .height(WINDOW_HEIGHT)
            .clip(true)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.background)),
                border: Border {
                    color: colors.border,
                    width: 1.0,
                    radius: Radius::from(theme::WINDOW_RADIUS),
                },
                ..Default::default()
            })
            .into()
    }

    /// Scrollable app list
    fn view_app_list(&self) -> Element<'_, Message> {
        let items: Vec<Element<Message>> = self
            .filtered_apps
            .iter()
            .enumerate()
            .map(|(idx, app)| self.view_app_item(idx, app))
            .collect();

        let list = Column::with_children(items).spacing(theme::APP_ITEM_SPACING);

        scrollable(list)
            .id(scrollable::Id::new("app_list"))
            .height(Fill)
            .style(|theme, status| {
                let mut style = scrollable::default(theme, status);
                // Hide the scrollbar by making rails invisible
                style.vertical_rail.background = None;
                style.vertical_rail.border = Border::default();
                style.vertical_rail.scroller.color = Color::TRANSPARENT;
                style.horizontal_rail.background = None;
                style.horizontal_rail.border = Border::default();
                style.horizontal_rail.scroller.color = Color::TRANSPARENT;
                style
            })
            .into()
    }

    /// Scroll to keep selected item visible
    fn scroll_to_selected(&self) -> Task<Message> {
        // Calculate the approximate y offset based on item height and spacing
        let item_height = theme::APP_ICON_SIZE + (theme::APP_ITEM_PADDING_V * 2.0);
        let total_height = item_height + theme::APP_ITEM_SPACING as f32;
        let offset = self.selected_index as f32 * total_height;

        // Scroll to that offset
        scrollable::scroll_to(
            scrollable::Id::new("app_list"),
            scrollable::AbsoluteOffset { x: 0.0, y: offset },
        )
    }

    /// Single app list item
    fn view_app_item<'a>(&'a self, index: usize, app: &'a AppEntry) -> Element<'a, Message> {
        let is_selected = index == self.selected_index;
        let selected_bg = self.colors.selected;

        // App icon (placeholder circle if no icon)
        let icon_element: Element<Message> = if let Some(ref icon_data) = app.icon_data {
            // If we have icon data, display it
            let handle = image::Handle::from_rgba(
                icon_data.width,
                icon_data.height,
                icon_data.rgba.clone(),
            );
            Image::new(handle)
                .width(theme::APP_ICON_SIZE)
                .height(theme::APP_ICON_SIZE)
                .content_fit(ContentFit::Contain)
                .into()
        } else {
            // Placeholder circle with first letter
            let first_char = app.name.chars().next().unwrap_or('?');
            container(
                text(first_char.to_string())
                    .size(14.0)
                    .style(|_| text::Style {
                        color: Some(Color::from_rgba(0.2, 0.2, 0.3, 1.0)),
                    })
            )
                .width(theme::APP_ICON_SIZE)
                .height(theme::APP_ICON_SIZE)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.85))),
                    border: Border {
                        radius: Radius::from(theme::APP_ICON_SIZE / 2.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        };

        // App name
        let name = text(&app.name)
            .size(theme::APP_NAME_FONT_SIZE)
            .style(|_| text::Style {
                color: Some(Color::WHITE),
            });

        let row_content = row![icon_element, name]
            .spacing(theme::APP_ICON_TEXT_GAP)
            .align_y(iced::Alignment::Center);

        let item = container(row_content)
            .padding([theme::APP_ITEM_PADDING_V as u16, theme::APP_ITEM_PADDING_H as u16])
            .width(Fill);

        // Use button for hover effect
        button(item)
            .on_press(Message::LaunchApp(index))
            .width(Fill)
            .style(move |_theme, status| {
                let bg_color = if is_selected {
                    selected_bg
                } else if matches!(status, button::Status::Hovered) {
                    Color::from_rgba(
                        theme::HOVER_BG.0,
                        theme::HOVER_BG.1,
                        theme::HOVER_BG.2,
                        theme::HOVER_BG.3,
                    )
                } else {
                    Color::TRANSPARENT
                };

                button::Style {
                    background: Some(Background::Color(bg_color)),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: Radius::from(theme::APP_ITEM_RADIUS),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    /// Subscriptions for keyboard events and IPC
    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = iced::event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                Some(Message::KeyPressed(key))
            }
            _ => None,
        });

        let ipc_sub = iced::Subscription::run(ipc_subscription);

        // Listen for window unfocus events
        let unfocus_sub = iced::event::listen_with(|event, _status, _id| match event {
            Event::Window(window::Event::Unfocused) => Some(Message::WindowUnfocused),
            _ => None,
        });

        Subscription::batch([keyboard_sub, ipc_sub, unfocus_sub])
    }

    /// Theme with transparent background for rounded corners
    pub fn theme(&self) -> Theme {
        // Use a custom palette with transparent background
        Theme::custom(
            "Transparent Dark".to_string(),
            iced::theme::Palette {
                background: Color::TRANSPARENT,
                text: Color::WHITE,
                primary: Color::from_rgb(0.3, 0.5, 0.8),
                success: Color::from_rgb(0.3, 0.8, 0.3),
                danger: Color::from_rgb(0.8, 0.3, 0.3),
            },
        )
    }
}

/// Launch an app path (or shell: URI) without flashing a console window.
/// `al` has no console, so a bare `cmd` spawn would open a new terminal
/// window that steals focus from the launcher.
fn launch_path(path: &str) {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn();
}

/// Stream IPC commands from the background thread
fn ipc_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::futures::stream::unfold(
        IpcState::NotStarted,
        |state| async move {
            match state {
                IpcState::NotStarted => {
                    // Try to get the receiver
                    if let Some(rx) = crate::take_ipc_receiver() {
                        Some((Message::FocusSearch, IpcState::Running(rx)))
                    } else {
                        // Wait and try again
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        Some((Message::FocusSearch, IpcState::NotStarted))
                    }
                }
                IpcState::Running(rx) => {
                    // Poll for commands
                    loop {
                        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                            Ok(cmd) => {
                                return Some((Message::IpcCommand(cmd), IpcState::Running(rx)));
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                // Yield to let other async tasks run
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                continue;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                // Channel closed - this shouldn't happen normally
                                return None;
                            }
                        }
                    }
                }
            }
        },
    )
}

/// State machine for IPC subscription
enum IpcState {
    NotStarted,
    Running(std::sync::mpsc::Receiver<crate::ipc::IpcCommand>),
}
