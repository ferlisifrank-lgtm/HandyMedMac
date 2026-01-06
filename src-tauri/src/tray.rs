use crate::settings;
use crate::tray_i18n::get_tray_translations;
use log::error;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Manager, Theme};

#[derive(Clone, Debug, PartialEq)]
pub enum TrayIconState {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppTheme {
    Dark,
    Light,
    Colored, // Pink/colored theme for Linux
}

/// Gets the current app theme, with Linux defaulting to Colored theme
pub fn get_current_theme(app: &AppHandle) -> AppTheme {
    if cfg!(target_os = "linux") {
        // On Linux, always use the colored theme
        AppTheme::Colored
    } else {
        // On other platforms, map system theme to our app theme
        if let Some(main_window) = app.get_webview_window("main") {
            match main_window.theme().unwrap_or(Theme::Dark) {
                Theme::Light => AppTheme::Light,
                Theme::Dark => AppTheme::Dark,
                _ => AppTheme::Dark, // Default fallback
            }
        } else {
            AppTheme::Dark
        }
    }
}

/// Gets the appropriate icon path for the given theme and state
pub fn get_icon_path(theme: AppTheme, state: TrayIconState) -> &'static str {
    match (theme, state) {
        // Dark theme uses light icons
        (AppTheme::Dark, TrayIconState::Idle) => "resources/tray_idle.png",
        (AppTheme::Dark, TrayIconState::Recording) => "resources/tray_recording.png",
        (AppTheme::Dark, TrayIconState::Transcribing) => "resources/tray_transcribing.png",
        // Light theme uses dark icons
        (AppTheme::Light, TrayIconState::Idle) => "resources/tray_idle_dark.png",
        (AppTheme::Light, TrayIconState::Recording) => "resources/tray_recording_dark.png",
        (AppTheme::Light, TrayIconState::Transcribing) => "resources/tray_transcribing_dark.png",
        // Colored theme uses pink icons (for Linux)
        (AppTheme::Colored, TrayIconState::Idle) => "resources/handy.png",
        (AppTheme::Colored, TrayIconState::Recording) => "resources/recording.png",
        (AppTheme::Colored, TrayIconState::Transcribing) => "resources/transcribing.png",
    }
}

pub fn change_tray_icon(app: &AppHandle, icon: TrayIconState) {
    let tray = app.state::<TrayIcon>();
    let theme = get_current_theme(app);

    let icon_path = get_icon_path(theme, icon.clone());

    let _ = tray.set_icon(Some(
        Image::from_path(
            app.path()
                .resolve(icon_path, tauri::path::BaseDirectory::Resource)
                .expect("failed to resolve"),
        )
        .expect("failed to set icon"),
    ));

    // Update menu based on state
    update_tray_menu(app, &icon, None);
}

pub fn update_tray_menu(app: &AppHandle, state: &TrayIconState, locale: Option<&str>) {
    if let Err(e) = try_update_tray_menu(app, state, locale) {
        error!("Failed to update tray menu: {}", e);
    }
}

fn try_update_tray_menu(
    app: &AppHandle,
    state: &TrayIconState,
    locale: Option<&str>,
) -> Result<(), String> {
    let settings = settings::get_settings(app);

    let locale = locale.unwrap_or(&settings.app_language);
    let strings = get_tray_translations(Some(locale.to_string()));

    // Platform-specific accelerators
    #[cfg(target_os = "macos")]
    let (settings_accelerator, quit_accelerator) = (Some("Cmd+,"), Some("Cmd+Q"));
    #[cfg(not(target_os = "macos"))]
    let (settings_accelerator, quit_accelerator) = (Some("Ctrl+,"), Some("Ctrl+Q"));

    // Create common menu items
    let version_label = if cfg!(debug_assertions) {
        format!("Handy v{} (Dev)", env!("CARGO_PKG_VERSION"))
    } else {
        format!("Handy v{}", env!("CARGO_PKG_VERSION"))
    };
    let version_i = MenuItem::with_id(app, "version", &version_label, false, None::<&str>)
        .map_err(|e| format!("Failed to create version menu item: {}", e))?;
    let settings_i = MenuItem::with_id(
        app,
        "settings",
        &strings.settings,
        true,
        settings_accelerator,
    )
    .map_err(|e| format!("Failed to create settings menu item: {}", e))?;
    // let check_updates_i = MenuItem::with_id(
    //     app,
    //     "check_updates",
    //     &strings.check_updates,
    //     settings.update_checks_enabled,
    //     None::<&str>,
    // )
    // .map_err(|e| format!("Failed to create check updates menu item: {}", e))?;
    let quit_i = MenuItem::with_id(app, "quit", &strings.quit, true, quit_accelerator)
        .map_err(|e| format!("Failed to create quit menu item: {}", e))?;
    let separator = || {
        PredefinedMenuItem::separator(app).map_err(|e| format!("Failed to create separator: {}", e))
    };

    let menu = match state {
        TrayIconState::Recording | TrayIconState::Transcribing => {
            let cancel_i = MenuItem::with_id(app, "cancel", &strings.cancel, true, None::<&str>)
                .map_err(|e| format!("Failed to create cancel menu item: {}", e))?;
            Menu::with_items(
                app,
                &[
                    &version_i,
                    &separator()?,
                    &cancel_i,
                    &separator()?,
                    &settings_i,
                    // &check_updates_i,
                    &separator()?,
                    &quit_i,
                ],
            )
            .map_err(|e| format!("Failed to create recording menu: {}", e))?
        }
        TrayIconState::Idle => Menu::with_items(
            app,
            &[
                &version_i,
                &separator()?,
                &settings_i,
                // &check_updates_i,
                &separator()?,
                &quit_i,
            ],
        )
        .map_err(|e| format!("Failed to create idle menu: {}", e))?,
    };

    let tray = app.state::<TrayIcon>();
    tray.set_menu(Some(menu))
        .map_err(|e| format!("Failed to set tray menu: {}", e))?;
    tray.set_icon_as_template(true)
        .map_err(|e| format!("Failed to set icon as template: {}", e))?;

    Ok(())
}
