use crate::config;
use crate::fl;
use crate::kdbx::{self, KpEntry};

use cosmic::iced::window::Id;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: config::Config,
    // Database state
    entries: Vec<KpEntry>,
    unlocked: bool,
    password_input: String,
    search_input: String,
    status_text: String,
    // Detail view
    detail_entry: Option<KpEntry>,
    // Auto-lock
    last_activity: Instant,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    PasswordInput(String),
    Unlock,
    Lock,
    SearchInput(String),
    CopyPassword(usize),
    CopyUsername(usize),
    ShowDetails(usize),
    CloseDetails,
    OpenSettings,
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
            unlocked: false,
            password_input: String::new(),
            search_input: String::new(),
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
        let icon = if self.unlocked {
            "channel-insecure-symbolic"
        } else {
            "channel-secure-symbolic"
        };

        self.core
            .applet
            .icon_button(icon)
            .on_press_down(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content: Element<'_, Self::Message> = if let Some(entry) = &self.detail_entry {
            // Detail view
            self.view_details(entry)
        } else if !self.unlocked {
            // Unlock view
            self.view_unlock()
        } else {
            // Entry list view
            self.view_entries()
        };

        let cosmic = self.core.system_theme().cosmic();
        let pad =
            cosmic::iced::Padding::from([cosmic.space_xxs() as u16, cosmic.space_xs() as u16]);

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

                    // Auto-lock check
                    if self.unlocked && self.config.auto_lock_minutes > 0 {
                        let elapsed = self.last_activity.elapsed().as_secs() / 60;
                        if elapsed >= self.config.auto_lock_minutes as u64 {
                            self.unlocked = false;
                            self.entries.clear();
                            self.password_input.clear();
                        }
                    }

                    self.status_text.clear();
                    self.detail_entry = None;

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    get_popup(popup_settings)
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
                match kdbx::open_database(&self.config.db_path, &self.password_input) {
                    Ok(entries) => {
                        self.entries = entries;
                        self.unlocked = true;
                        self.status_text.clear();
                        self.password_input.clear();
                    }
                    Err(e) => {
                        self.status_text = fl!("unlock-error", error = e.as_str());
                    }
                }
            }
            Message::Lock => {
                self.unlocked = false;
                self.entries.clear();
                self.password_input.clear();
                self.search_input.clear();
                self.detail_entry = None;
            }
            Message::SearchInput(q) => {
                self.search_input = q;
            }
            Message::CopyPassword(idx) => {
                if let Some(entry) = self.filtered_entries().get(idx) {
                    copy_to_clipboard(&entry.password);
                    self.status_text = fl!("copied");
                }
            }
            Message::CopyUsername(idx) => {
                if let Some(entry) = self.filtered_entries().get(idx) {
                    copy_to_clipboard(&entry.username);
                    self.status_text = fl!("copied");
                }
            }
            Message::ShowDetails(idx) => {
                if let Some(entry) = self.filtered_entries().get(idx) {
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
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn filtered_entries(&self) -> Vec<KpEntry> {
        if self.search_input.is_empty() {
            return self.entries.clone();
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
        let pw_field = widget::text_input(fl!("master-password-placeholder"), &self.password_input)
            .on_input(Message::PasswordInput)
            .on_submit(|_| Message::Unlock)
            .password();

        let unlock_btn = cosmic::applet::menu_button(
            widget::row::with_children(vec![
                widget::icon::from_name("channel-insecure-symbolic")
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

    fn view_entries(&self) -> Element<'_, Message> {
        let search = widget::text_input(fl!("search-placeholder"), &self.search_input)
            .on_input(Message::SearchInput);

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

        let filtered = self.filtered_entries();

        let entry_items: Vec<Element<'_, Message>> = if filtered.is_empty() {
            vec![widget::text(fl!("no-entries")).size(13).into()]
        } else {
            filtered
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let title = widget::text(entry.title.clone()).size(14);
                    let subtitle = widget::text(entry.username.clone()).size(11);

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

                    let left = widget::column::with_children(vec![title.into(), subtitle.into()])
                        .spacing(2);

                    let buttons =
                        widget::row::with_children(vec![
                            pw_btn.into(),
                            user_btn.into(),
                            detail_btn.into(),
                        ])
                        .spacing(4);

                    widget::row::with_children(vec![
                        left.into(),
                        widget::Space::new().width(cosmic::iced::Length::Fill).into(),
                        buttons.into(),
                    ])
                    .spacing(8)
                    .into()
                })
                .collect()
        };

        let entry_list =
            widget::scrollable(widget::column::with_children(entry_items).spacing(6));

        let mut items: Vec<Element<'_, Message>> = vec![
            search.into(),
            widget::divider::horizontal::default().into(),
        ];

        items.push(entry_list.into());

        items.push(widget::divider::horizontal::default().into());

        let bottom = widget::row::with_children(vec![lock_btn.into(), settings_btn.into()])
            .spacing(0);
        items.push(bottom.into());

        if !self.status_text.is_empty() {
            items.push(widget::text(&self.status_text).size(12).into());
        }

        widget::column::with_children(items).spacing(4).into()
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
            ]).spacing(2).into(),
            widget::column::with_children(vec![
                widget::text(fl!("details-password")).size(11).into(),
                widget::text("••••••••").size(14).into(),
            ]).spacing(2).into(),
            widget::column::with_children(vec![
                widget::text(fl!("details-url")).size(11).into(),
                widget::text(url).size(14).into(),
            ]).spacing(2).into(),
        ];

        if !notes.is_empty() {
            items.push(
                widget::column::with_children(vec![
                    widget::text(fl!("details-notes")).size(11).into(),
                    widget::text(notes).size(14).into(),
                ]).spacing(2).into(),
            );
        }

        items.push(widget::divider::horizontal::default().into());

        let close_btn = cosmic::applet::menu_button(
            widget::text(fl!("details-close")).size(14),
        )
        .on_press(Message::CloseDetails);

        items.push(close_btn.into());

        widget::column::with_children(items).spacing(6).into()
    }
}
