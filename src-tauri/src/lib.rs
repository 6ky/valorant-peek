mod auth;
mod client_version;
mod fetcher;
mod http;
mod lockfile;
mod match_state;
mod model;
mod orchestrator;
mod static_cache;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(orchestrator::run_loop(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
