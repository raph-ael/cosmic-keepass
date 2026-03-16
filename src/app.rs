use crate::config;
use crate::fl;
use crate::kdbx::{self, KpEntry};

use cosmic::iced::window::Id;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;

const SEARCH_INPUT_ID: &str = "keepass-search";

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Locked,
    Unlocking,
    Unlocked,
}

pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: config::Config,
    entries: Vec<KpEntry>,
    lock_state: LockState,
    password_input: String,
    search_input: String,
    show_all: bool,
    status_text: String,
    detail_entry: Option<KpEntry>,
    last_activity: Instant,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    PasswordInput(String),
    Unlock,
    UnlockDone(Result<Vec<KpEntry>, String>),
    Lock,
    SearchInput(String),
    ToggleShowAll,
    CopyPassword(usize),
    CopyUsername(usize),
    ShowDetails(usize),
    CloseDetails,
    OpenSettings,
    OpenNewEntry,
    FocusSearch,
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut child) = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic-keepass";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let cfg = config::load_config();
        let app = AppModel {
            core,
            popup: None,
            config: cfg,
            entries: Vec::new(),
            lock_state: LockState::Locked,
            password_input: String::new(),
            search_input: String::new(),
            show_all: false,
            status_text: String::new(),
            detail_entry: None,
            last_activity: Instant::now(),
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let icon = if self.lock_state == LockState::Unlocked {
            "changes-allow-symbolic"
        } else {
            "changes-prevent-symbolic"
        };

        self.core
            .applet
            .icon_button(icon)
            .on_press_down(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content: Element<'_, Self::Message> = if let Some(entry) = &self.detail_entry {
            self.view_details(entry)
        } else {
            match self.lock_state {
                LockState::Locked => self.view_unlock(),
                LockState::Unlocking => self.view_unlocking(),
                LockState::Unlocked => self.view_entries(),
            }
        };

        let cosmic = self.core.system_theme().cosmic();
        let pad = cosmic::iced::Padding::from([cosmic.space_xxs() as u16, 0]);

        self.core
            .applet
            .popup_container(widget::container(content).padding(pad))
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        self.last_activity = Instant::now();

        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.detail_entry = None;
                    destroy_popup(p)
                } else {
                    self.config = config::load_config();

                    if self.lock_state == LockState::Unlocked
                        && self.config.auto_lock_minutes > 0
                    {
                        let elapsed = self.last_activity.elapsed().as_secs() / 60;
                        if elapsed >= self.config.auto_lock_minutes as u64 {
                            self.lock_state = LockState::Locked;
                            self.entries.clear();
                            self.password_input.clear();
                        }
                    }

                    self.status_text.clear();
                    self.detail_entry = None;
                    self.show_all = false;
                    self.search_input.clear();

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    let popup_task = get_popup(popup_settings);

                    // Schedule focus after popup renders
                    if self.lock_state == LockState::Unlocked {
                        let focus_task = Task::perform(
                            async { tokio::time::sleep(std::time::Duration::from_millis(200)).await },
                            |_| cosmic::Action::App(Message::FocusSearch),
                        );
                        return Task::batch(vec![popup_task, focus_task]);
                    }

                    popup_task
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.detail_entry = None;
                }
            }
            Message::PasswordInput(pw) => {
                self.password_input = pw;
            }
            Message::Unlock => {
                if self.config.db_path.is_empty() {
                    self.status_text = fl!("db-not-configured");
                    return Task::none();
                }
                self.lock_state = LockState::Unlocking;
                self.status_text.clear();

                let db_path = self.config.db_path.clone();
                let password = self.password_input.clone();

                return Task::perform(
                    async move {
                        // Run blocking DB open in a thread
                        tokio::task::spawn_blocking(move || {
                            kdbx::open_database(&db_path, &password)
                        })
                        .await
                        .map_err(|e| format!("{e}"))?
                    },
                    |result| cosmic::Action::App(Message::UnlockDone(result)),
                );
            }
            Message::UnlockDone(result) => match result {
                Ok(entries) => {
                    self.entries = entries;
                    self.lock_state = LockState::Unlocked;
                    self.status_text.clear();
                    self.password_input.clear();
                }
                Err(e) => {
                    self.lock_state = LockState::Locked;
                    self.status_text = fl!("unlock-error", error = e.as_str());
                }
            },
            Message::Lock => {
                self.lock_state = LockState::Locked;
                self.entries.clear();
                self.password_input.clear();
                self.search_input.clear();
                self.show_all = false;
                self.detail_entry = None;
            }
            Message::SearchInput(q) => {
                self.search_input = q;
                self.show_all = false;
            }
            Message::ToggleShowAll => {
                self.show_all = !self.show_all;
            }
            Message::CopyPassword(idx) => {
                if let Some(entry) = self.visible_entries().get(idx) {
                    copy_to_clipboard(&entry.password);
                    self.search_input = fl!("copied");
                }
            }
            Message::CopyUsername(idx) => {
                if let Some(entry) = self.visible_entries().get(idx) {
                    copy_to_clipboard(&entry.username);
                    self.search_input = fl!("copied");
                }
            }
            Message::ShowDetails(idx) => {
                if let Some(entry) = self.visible_entries().get(idx) {
                    self.detail_entry = Some(entry.clone());
                }
            }
            Message::CloseDetails => {
                self.detail_entry = None;
            }
            Message::OpenSettings => {
                let _ = Command::new("cosmic-keepass")
                    .arg("--settings")
                    .spawn();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::FocusSearch => {
                return widget::text_input::focus(widget::Id::new(SEARCH_INPUT_ID));
            }
            Message::OpenNewEntry => {
                let _ = Command::new("cosmic-keepass")
                    .arg("--new-entry")
                    .spawn();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn visible_entries(&self) -> Vec<KpEntry> {
        if self.show_all {
            return self.entries.clone();
        }
        if self.search_input.is_empty() {
            return Vec::new();
        }
        let q = self.search_input.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&q)
                    || e.username.to_lowercase().contains(&q)
                    || e.url.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    fn view_unlock(&self) -> Element<'_, Message> {
        let cosmic_theme = self.core.system_theme().cosmic();
        let hpad = cosmic_theme.space_xs() as u16;

        let pw_field = widget::container(
            widget::text_input(fl!("master-password-placeholder"), &self.password_input)
                .on_input(Message::PasswordInput)
                .on_submit(|_| Message::Unlock)
                .password(),
        )
        .padding([0, hpad]);

        let unlock_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("changes-allow-symbolic")
                    .size(16)
                    .icon()
                    .into(),
                widget::text(fl!("unlock")).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::Unlock);

        let settings_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("emblem-system-symbolic")
                    .size(16)
                    .icon()
                    .into(),
                widget::text(fl!("settings")).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::OpenSettings);

        let mut items: Vec<Element<'_, Message>> = vec![
            pw_field.into(),
            unlock_btn.into(),
            widget::divider::horizontal::default().into(),
            settings_btn.into(),
        ];

        if !self.status_text.is_empty() {
            items.push(widget::text(&self.status_text).size(12).into());
        }

        widget::column::with_children(items).spacing(4).into()
    }

    fn view_unlocking(&self) -> Element<'_, Message> {
        let cosmic_theme = self.core.system_theme().cosmic();
        let hpad = cosmic_theme.space_xs() as u16;

        let loading_field = widget::container(
            widget::text_input(fl!("unlocking"), ""),
        )
        .padding([0, hpad]);

        widget::column::with_children(vec![loading_field.into()])
            .spacing(4)
            .into()
    }

    fn view_entries(&self) -> Element<'_, Message> {
        // Top: new entry button
        let new_entry_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("list-add-symbolic")
                    .size(16)
                    .icon()
                    .into(),
                widget::text(fl!("new-entry")).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::OpenNewEntry);

        // Search (with horizontal padding)
        let cosmic_theme = self.core.system_theme().cosmic();
        let hpad = cosmic_theme.space_xs() as u16;
        let search = widget::container(
            widget::text_input(fl!("search-placeholder"), &self.search_input)
                .on_input(Message::SearchInput)
                .id(widget::Id::new(SEARCH_INPUT_ID)),
        )
        .padding([0, hpad]);

        // Show/hide all toggle
        let toggle_label = if self.show_all {
            fl!("hide-all")
        } else {
            fl!("show-all")
        };
        let toggle_icon = if self.show_all {
            "view-conceal-symbolic"
        } else {
            "view-list-symbolic"
        };
        let show_all_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name(toggle_icon)
                    .size(16)
                    .icon()
                    .into(),
                widget::text(toggle_label).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::ToggleShowAll);

        // Bottom buttons
        let lock_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("system-lock-screen-symbolic")
                    .size(16)
                    .icon()
                    .into(),
                widget::text(fl!("lock")).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::Lock);

        let settings_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("emblem-system-symbolic")
                    .size(16)
                    .icon()
                    .into(),
                widget::text(fl!("settings")).size(14).into(),
            ])
            .spacing(8),
        )
        .on_press(Message::OpenSettings);

        let mut items: Vec<Element<'_, Message>> = vec![search.into()];

        // Entry list directly below search
        if !self.search_input.is_empty() || self.show_all {
            let visible = self.visible_entries();
            let entry_items: Vec<Element<'_, Message>> = if visible.is_empty() {
                vec![
                    cosmic::applet::menu_button(widget::text(fl!("no-entries")).size(14)).into(),
                ]
            } else {
                visible
                    .iter()
                    .enumerate()
                    .map(|(idx, entry)| {
                        let mut title = entry.title.clone();
                        if title.len() > 25 {
                            title.truncate(22);
                            title.push_str("...");
                        }

                        let pw_btn = widget::button::icon(
                            widget::icon::from_name("dialog-password-symbolic").size(14),
                        )
                        .on_press(Message::CopyPassword(idx));

                        let user_btn = widget::button::icon(
                            widget::icon::from_name("system-users-symbolic").size(14),
                        )
                        .on_press(Message::CopyUsername(idx));

                        let detail_btn = widget::button::icon(
                            widget::icon::from_name("view-more-symbolic").size(14),
                        )
                        .on_press(Message::ShowDetails(idx));

                        let buttons = widget::row::with_children(vec![
                            pw_btn.into(),
                            user_btn.into(),
                            detail_btn.into(),
                        ])
                        .spacing(4)
                        .align_y(cosmic::iced::Alignment::Center);

                        cosmic::applet::menu_button(
                            widget::row::with_children(vec![
                                widget::text(title).size(14).into(),
                                widget::Space::new()
                                    .width(cosmic::iced::Length::Fill)
                                    .into(),
                                buttons.into(),
                            ])
                            .spacing(8)
                            .align_y(cosmic::iced::Alignment::Center),
                        )
                        .into()
                    })
                    .collect()
            };

            items.push(
                widget::scrollable(widget::column::with_children(entry_items)).into(),
            );
        }

        items.push(new_entry_btn.into());
        items.push(show_all_btn.into());
        items.push(widget::divider::horizontal::default().into());
        items.push(lock_btn.into());
        items.push(settings_btn.into());

        widget::column::with_children(items).spacing(0).into()
    }

    fn view_details(&self, entry: &KpEntry) -> Element<'_, Message> {
        let title = widget::text::title4(entry.title.clone());
        let username = entry.username.clone();
        let url = entry.url.clone();
        let notes = entry.notes.clone();

        let mut items: Vec<Element<'_, Message>> = vec![
            title.into(),
            widget::divider::horizontal::default().into(),
            widget::column::with_children(vec![
                widget::text(fl!("details-username")).size(11).into(),
                widget::text(username).size(14).into(),
            ])
            .spacing(2)
            .into(),
            widget::column::with_children(vec![
                widget::text(fl!("details-password")).size(11).into(),
                widget::text("••••••••").size(14).into(),
            ])
            .spacing(2)
            .into(),
            widget::column::with_children(vec![
                widget::text(fl!("details-url")).size(11).into(),
                widget::text(url).size(14).into(),
            ])
            .spacing(2)
            .into(),
        ];

        if !notes.is_empty() {
            items.push(
                widget::column::with_children(vec![
                    widget::text(fl!("details-notes")).size(11).into(),
                    widget::text(notes).size(14).into(),
                ])
                .spacing(2)
                .into(),
            );
        }

        items.push(widget::divider::horizontal::default().into());

        let close_btn =
            cosmic::applet::menu_button(widget::text(fl!("details-close")).size(14))
                .on_press(Message::CloseDetails);

        items.push(close_btn.into());

        widget::column::with_children(items).spacing(6).into()
    }
}
