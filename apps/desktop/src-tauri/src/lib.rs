use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    App, Manager, RunEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_shell::ShellExt;

mod sidecar;
mod tray;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            sidecar::get_service_status,
            sidecar::restart_api_server,
        ])
        .setup(|app| {
            tray::setup_tray(app)?;
            sidecar::launch_services(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                sidecar::shutdown_services(&app_handle);
                api.prevent_exit();
            }
            RunEvent::Exit => {
                sidecar::shutdown_services(&app_handle);
            }
            _ => {}
        });
}
