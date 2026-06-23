// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

pub mod log;
pub mod dhcp;
pub mod udp;
pub mod tcp;
pub mod http;
pub mod someip;

#[tauri::command]
fn greet(name: &str) -> String {
    let greeting = format!("Hello, {}! You've been greeted from Rust!", name);
    log_info!("Greet command called with name: {}", name);
    greeting
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志系统
    if let Err(e) = log::init_logger_default() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    log_info!("Tauri application starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}