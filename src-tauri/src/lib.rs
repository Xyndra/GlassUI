

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub fn run(show_settings: bool) {
    let valid_cfg = cfg!(target_os = "windows") && cfg!(desktop);
    if !valid_cfg {
        println!("This system is not supported by this application.");
        return;
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // TODO: open settings
        }))
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // TODO: Create the windows

    app.run(|_app_handle, _event| {})
}