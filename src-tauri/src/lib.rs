pub mod log;
pub mod dhcp;
pub mod udp;
pub mod tcp;
pub mod http;
pub mod someip;
pub mod bcm;
pub mod render;
pub mod rtp;

use bcm::error::BcmError;
use bcm::types::ConfigPair;
use serde::Serialize;

#[derive(Serialize)]
pub struct BcmResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> BcmResult<T> {
    fn ok(data: T) -> Self {
        BcmResult {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

fn err_msg(e: BcmError) -> BcmResult<String> {
    BcmResult {
        success: false,
        data: None,
        error: Some(e.to_string()),
    }
}

#[tauri::command]
async fn reboot(device_ip: String) -> BcmResult<String> {
    match bcm::tool::reboot(&device_ip).await {
        Ok(_) => BcmResult::ok("ok".into()),
        Err(e) => err_msg(e),
    }
}

#[tauri::command]
async fn get_version(device_ip: String) -> BcmResult<String> {
    match bcm::tool::get_version(&device_ip).await {
        Ok(ver) => BcmResult::ok(ver),
        Err(e) => err_msg(e),
    }
}

#[tauri::command]
async fn read_config(device_ip: String) -> BcmResult<Vec<ConfigPair>> {
    match bcm::tool::read_config(&device_ip).await {
        Ok(pairs) => BcmResult::ok(pairs),
        Err(e) => BcmResult {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
async fn health_check(device_ip: String, pid: u16) -> BcmResult<String> {
    match bcm::tool::health_check(&device_ip, pid).await {
        Ok(info) => BcmResult::ok(info),
        Err(e) => err_msg(e),
    }
}

#[tauri::command]
async fn full_install(
    file_name: String,
    file_bytes: Vec<u8>,
    device_ip: String,
    host_ip: String,
) -> BcmResult<String> {
    match bcm::tool::full_install(&file_name, file_bytes, &device_ip, &host_ip).await {
        Ok(_) => BcmResult::ok("ok".into()),
        Err(e) => err_msg(e),
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    let greeting = format!("Hello, {}! You've been greeted from Rust!", name);
    log_info!("Greet command called with name: {}", name);
    greeting
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = log::init_logger_default() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    log_info!("Tauri application starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            reboot,
            get_version,
            read_config,
            health_check,
            full_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
