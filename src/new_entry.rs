use crate::config;
use crate::fl;
use crate::kdbx;

use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget;

pub struct NewEntryModel {
    core: cosmic::Core,
    config: config::Config,
    master_password: String,
    title: String,
    username: String,
    password: String,
    url: String,
    notes: String,
    status: String,
    authenticated: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    MasterPasswordInput(String),
    Authenticate,
    TitleInput(String),
    UsernameInput(String),
    PasswordInput(String),
    UrlInput(String),
    NotesInput(String),
    Save,
    Close,
}

impl cosmic::Application for NewEntryModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic-keepass.new-entry";

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
        let app = NewEntryModel {
            core,
            config: cfg,
            master_password: String::new(),
            title: String::new(),
            username: String::new(),
            password: String::new(),
            url: String::new(),
            notes: String::new(),
            status: String::new(),
            authenticated: false,
        };
        (app, Task::none())
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let title = widget::text::title3(fl!("new-entry-title"));

        let content: Element<'_, Self::Message> = if !self.authenticated {
            // Master password step
            let pw_label = widget::text(fl!("master-password-placeholder"));
            let pw_field = widget::text_input("", &self.master_password)
                .on_input(Message::MasterPasswordInput)
                .on_submit(|_| Message::Authenticate)
                .password()
                .width(Length::Fill);
            let auth_btn =
                widget::button::suggested(fl!("unlock")).on_press(Message::Authenticate);

            widget::column::with_children(vec![
                pw_label.into(),
                pw_field.into(),
                auth_btn.into(),
            ])
            .spacing(12)
            .into()
        } else {
            // Entry form
            let title_field = widget::text_input(fl!("entry-title-placeholder"), &self.title)
                .on_input(Message::TitleInput)
                .width(Length::Fill);

            let user_field = widget::text_input(fl!("entry-username-placeholder"), &self.username)
                .on_input(Message::UsernameInput)
                .width(Length::Fill);

            let pw_field = widget::text_input(fl!("entry-password-placeholder"), &self.password)
                .on_input(Message::PasswordInput)
                .password()
                .width(Length::Fill);

            let url_field = widget::text_input(fl!("entry-url-placeholder"), &self.url)
                .on_input(Message::UrlInput)
                .width(Length::Fill);

            let notes_field = widget::text_input(fl!("entry-notes-placeholder"), &self.notes)
                .on_input(Message::NotesInput)
                .width(Length::Fill);

            let save_btn = widget::button::suggested(fl!("save")).on_press(Message::Save);
            let close_btn = widget::button::standard(fl!("close")).on_press(Message::Close);

            widget::column::with_children(vec![
                widget::text(fl!("entry-title-label")).size(12).into(),
                title_field.into(),
                widget::text(fl!("details-username")).size(12).into(),
                user_field.into(),
                widget::text(fl!("details-password")).size(12).into(),
                pw_field.into(),
                widget::text(fl!("details-url")).size(12).into(),
                url_field.into(),
                widget::text(fl!("details-notes")).size(12).into(),
                notes_field.into(),
                widget::row::with_children(vec![save_btn.into(), close_btn.into()])
                    .spacing(8)
                    .into(),
            ])
            .spacing(8)
            .into()
        };

        let mut items: Vec<Element<'_, Self::Message>> = vec![title.into(), content];

        if !self.status.is_empty() {
            items.push(widget::text(&self.status).size(12).into());
        }

        let col = widget::column::with_children(items)
            .spacing(12)
            .padding(24)
            .width(Length::Fill);

        widget::container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::MasterPasswordInput(pw) => self.master_password = pw,
            Message::Authenticate => {
                // Verify master password by trying to open the database
                match kdbx::open_database(&self.config.db_path, &self.master_password) {
                    Ok(_) => {
                        self.authenticated = true;
                        self.status.clear();
                    }
                    Err(e) => {
                        self.status = fl!("unlock-error", error = e.as_str());
                    }
                }
            }
            Message::TitleInput(v) => self.title = v,
            Message::UsernameInput(v) => self.username = v,
            Message::PasswordInput(v) => self.password = v,
            Message::UrlInput(v) => self.url = v,
            Message::NotesInput(v) => self.notes = v,
            Message::Save => {
                if self.title.is_empty() {
                    self.status = fl!("entry-title-required");
                    return Task::none();
                }
                match kdbx::add_entry(
                    &self.config.db_path,
                    &self.master_password,
                    &self.title,
                    &self.username,
                    &self.password,
                    &self.url,
                    &self.notes,
                ) {
                    Ok(()) => {
                        self.status = fl!("entry-saved");
                        // Clear form for next entry
                        self.title.clear();
                        self.username.clear();
                        self.password.clear();
                        self.url.clear();
                        self.notes.clear();
                    }
                    Err(e) => {
                        self.status = e;
                    }
                }
            }
            Message::Close => {
                std::process::exit(0);
            }
        }
        Task::none()
    }
}
