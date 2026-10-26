use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
pub struct TrayState {
    open: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
    exiting: AtomicBool,
}
#[cfg(target_os = "macos")]
pub fn is_exiting(app: &tauri::AppHandle) -> bool {
    app.state::<TrayState>().exiting.load(Ordering::SeqCst)
}
pub fn restore(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
pub fn exit(app: tauri::AppHandle) {
    shutdown(app, None)
}
pub fn exit_for_update(app: tauri::AppHandle, path: std::path::PathBuf) {
    shutdown(app, Some(path))
}
fn shutdown(app: tauri::AppHandle, installer: Option<std::path::PathBuf>) {
    if app
        .state::<TrayState>()
        .exiting
        .swap(true, Ordering::SeqCst)
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let state = app.state::<crate::AppState>();
        let _ = crate::cancel_search(state.clone());
        state.discovery.pause().await;
        state.sharing.shutdown().await;
        state.v2_downloads.shutdown();
        state.downloads.shutdown().await;
        state.ed2k_downloads.shutdown().await;
        state.engine.shutdown().await;
        if let Some(path) = installer {
            if std::process::Command::new(path).spawn().is_err() {
                app.state::<TrayState>()
                    .exiting
                    .store(false, Ordering::SeqCst);
                restore(&app);
                return;
            }
        }
        app.exit(0);
    });
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "show", "Abrir MAGNETRON", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("MAGNETRON")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => restore(app),
            "quit" => exit(app.clone()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                restore(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayState {
        open,
        quit,
        exiting: AtomicBool::new(false),
    });
    Ok(())
}
#[tauri::command]
pub fn set_ui_language(language: String, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<TrayState>();
    let en = language == "en";
    state
        .open
        .set_text(if en {
            "Open MAGNETRON"
        } else {
            "Abrir MAGNETRON"
        })
        .map_err(|e| e.to_string())?;
    state
        .quit
        .set_text(if en { "Quit" } else { "Salir" })
        .map_err(|e| e.to_string())?;
    Ok(())
}
