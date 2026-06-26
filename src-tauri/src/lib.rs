mod auth;
mod client_version;
mod discord;
mod fetcher;
mod http;
mod lockfile;
mod match_state;
mod model;
mod orchestrator;
mod presence;
mod static_cache;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

struct AppState {
    rpc_enabled: Arc<AtomicBool>,
}

#[tauri::command]
fn set_rpc_enabled(state: tauri::State<AppState>, enabled: bool) {
    state.rpc_enabled.store(enabled, Ordering::Relaxed);
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Peek")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rpc_enabled = Arc::new(AtomicBool::new(true));
    let loop_flag = rpc_enabled.clone();

    tauri::Builder::default()
        .manage(AppState { rpc_enabled })
        .invoke_handler(tauri::generate_handler![set_rpc_enabled])
        .setup(move |app| {
            build_tray(app)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(orchestrator::run_loop(handle, loop_flag));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
