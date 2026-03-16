mod app;
mod config;
mod i18n;
mod kdbx;
mod settings;

fn main() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    if std::env::args().any(|a| a == "--settings") {
        let settings = cosmic::app::Settings::default()
            .size(cosmic::iced::Size::new(500.0, 450.0));
        cosmic::app::run::<settings::SettingsModel>(settings, ())
    } else {
        cosmic::applet::run::<app::AppModel>(())
    }
}
