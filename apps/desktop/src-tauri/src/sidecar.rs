use std::sync::Mutex;
use tauri::{App, AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;

struct ManagedChild {
    child: Option<tauri_plugin_shell::process::CommandChild>,
}

pub struct SidecarState {
    api_server: Mutex<Option<ManagedChild>>,
    agent_runner: Mutex<Option<ManagedChild>>,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self {
            api_server: Mutex::new(None),
            agent_runner: Mutex::new(None),
        }
    }
}

pub fn launch_services(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(SidecarState::default());

    let (_rx, child) = app
        .shell()
        .sidecar("binaries/api-server")
        .expect("failed to find api-server sidecar binary")
        .args(["--port", "3000"])
        .spawn()?;

    let state: State<SidecarState> = app.state();
    *state.api_server.lock().unwrap() = Some(ManagedChild {
        child: Some(child),
    });

    log::info!("API server sidecar launched on port 3000");
    Ok(())
}

fn kill_child(mc: &mut ManagedChild) {
    if let Some(child) = mc.child.take() {
        let _ = child.kill();
    }
}

pub fn shutdown_services(app: &AppHandle) {
    let state: State<SidecarState> = app.state();
    {
        if let Ok(mut guard) = state.api_server.lock() {
            if let Some(ref mut mc) = *guard {
                kill_child(mc);
                log::info!("API server sidecar shut down");
            }
            *guard = None;
        }
    };
    {
        if let Ok(mut guard) = state.agent_runner.lock() {
            if let Some(ref mut mc) = *guard {
                kill_child(mc);
                log::info!("Agent runner sidecar shut down");
            }
            *guard = None;
        }
    };
}

#[tauri::command]
pub fn restart_api_server(app: AppHandle) -> Result<String, String> {
    shutdown_services(&app);
    let (_rx, child) = app
        .shell()
        .sidecar("binaries/api-server")
        .map_err(|e| e.to_string())?
        .args(["--port", "3000"])
        .spawn()
        .map_err(|e| e.to_string())?;

    let state: State<SidecarState> = app.state();
    *state.api_server.lock().unwrap() = Some(ManagedChild {
        child: Some(child),
    });
    Ok("API server restarted".to_string())
}

#[tauri::command]
pub fn get_service_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let state: State<SidecarState> = app.state();
    let api_running = state
        .api_server
        .lock()
        .unwrap()
        .as_ref()
        .map_or(false, |mc| mc.child.is_some());
    let agent_running = state
        .agent_runner
        .lock()
        .unwrap()
        .as_ref()
        .map_or(false, |mc| mc.child.is_some());
    Ok(serde_json::json!({
        "apiServer": api_running,
        "agentRunner": agent_running
    }))
}
