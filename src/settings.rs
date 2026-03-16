use crate::config;
use crate::fl;
use crate::kdbx;

use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget;

pub struct SettingsModel {
    core: cosmic::Core,
    config: config::Config,
    db_path_input: String,
    new_password_input: String,
    auto_lock_input: String,
    saved: bool,
    status: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    DbPathChanged(String),
    NewPasswordChanged(String),
    AutoLockChanged(String),
    PastePath,
    CreateDatabase,
    Save,
    Close,
}

impl cosmic::Application for SettingsModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic-keepass.settings";

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
        let app = SettingsModel {
            core,
            db_path_input: cfg.db_path.clone(),
            new_password_input: String::new(),
            auto_lock_input: cfg.auto_lock_minutes.to_string(),
            saved: false,
            status: String::new(),
            config: cfg,
        };
        (app, Task::none())
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let title = widget::text::title3(fl!("settings-title"));

        // Database path
        let db_label = widget::text(fl!("db-path-label"));
        let db_field = widget::text_input(fl!("db-path-placeholder"), &self.db_path_input)
            .on_input(Message::DbPathChanged)
            .width(Length::Fill);
        let paste_btn =
            widget::button::standard(fl!("browse")).on_press(Message::PastePath);

        // Auto-lock
        let lock_label = widget::text(fl!("auto-lock-label"));
        let lock_field = widget::text_input("5", &self.auto_lock_input)
            .on_input(Message::AutoLockChanged)
            .width(Length::Fixed(80.0));

        // Create new database
        let create_label = widget::text(fl!("create-new-db"));
        let new_pw_field = widget::text_input(
            fl!("new-master-password-placeholder"),
            &self.new_password_input,
        )
        .on_input(Message::NewPasswordChanged)
        .password()
        .width(Length::Fill);

        let create_btn = if self.db_path_input.is_empty() || self.new_password_input.is_empty() {
            widget::button::suggested(fl!("create"))
        } else {
            widget::button::suggested(fl!("create")).on_press(Message::CreateDatabase)
        };

        // Save/Close
        let save_btn = widget::button::suggested(fl!("save")).on_press(Message::Save);
        let close_btn = widget::button::standard(fl!("close")).on_press(Message::Close);

        let status: Element<'_, Self::Message> = if !self.status.is_empty() {
            widget::text(&self.status).size(12).into()
        } else if self.saved {
            widget::text(fl!("status-saved")).size(12).into()
        } else {
            widget::text("").into()
        };

        let content = widget::column::with_children(vec![
            title.into(),
            widget::divider::horizontal::default().into(),
            db_label.into(),
            db_field.into(),
            paste_btn.into(),
            widget::divider::horizontal::default().into(),
            lock_label.into(),
            lock_field.into(),
            widget::divider::horizontal::default().into(),
            create_label.into(),
            new_pw_field.into(),
            create_btn.into(),
            widget::divider::horizontal::default().into(),
            widget::row::with_children(vec![save_btn.into(), close_btn.into()])
                .spacing(8)
                .into(),
            status,
        ])
        .spacing(12)
        .padding(24)
        .width(Length::Fill);

        widget::scrollable(
            widget::container(content)
                .width(Length::Fill)
                .height(Length::Shrink),
        )
        .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::DbPathChanged(path) => {
                self.db_path_input = path;
                self.saved = false;
            }
            Message::NewPasswordChanged(pw) => {
                self.new_password_input = pw;
            }
            Message::AutoLockChanged(val) => {
                self.auto_lock_input = val;
                self.saved = false;
            }
            Message::PastePath => {
                if let Ok(output) = std::process::Command::new("wl-paste")
                    .arg("--no-newline")
                    .output()
                {
                    if output.status.success() {
                        if let Ok(text) = String::from_utf8(output.stdout) {
                            self.db_path_input = text.trim().to_string();
                            self.saved = false;
                        }
                    }
                }
            }
            Message::CreateDatabase => {
                match kdbx::create_database(&self.db_path_input, &self.new_password_input) {
                    Ok(()) => {
                        self.status = fl!("db-created");
                        self.new_password_input.clear();
                    }
                    Err(e) => {
                        self.status = e;
                    }
                }
            }
            Message::Save => {
                self.config.db_path = self.db_path_input.clone();
                self.config.auto_lock_minutes =
                    self.auto_lock_input.parse().unwrap_or(5);
                config::save_config(&self.config);
                self.saved = true;
                self.status.clear();
            }
            Message::Close => {
                std::process::exit(0);
            }
        }
        Task::none()
    }
}
