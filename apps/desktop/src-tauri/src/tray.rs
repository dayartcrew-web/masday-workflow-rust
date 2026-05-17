use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "Show Dashboard").build(app)?;
    let restart = MenuItemBuilder::with_id("restart_api", "Restart API Server").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&restart)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .tooltip("Masday Workflow")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "restart_api" => {
                if let Err(e) = crate::sidecar::restart_api_server(app.clone()) {
                    log::error!("Failed to restart API server: {}", e);
                }
            }
            "quit" => {
                crate::sidecar::shutdown_services(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
