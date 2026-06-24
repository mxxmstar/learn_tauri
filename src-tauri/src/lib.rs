pub mod log;
pub mod dhcp;
pub mod udp;
pub mod tcp;
pub mod http;
pub mod httpclient;
pub mod someip;
pub mod bcm;
pub mod render;
pub mod rtp;
pub mod onvif;  // ONVIF 模块（设备发现、设备管理、设备能力）
pub mod pcap;   // pcap 网络数据包捕获模块（网卡枚举、实时抓包）
pub mod stonkam_avtp;  // Stonkam 自定义 AVTP 协议解析模块（EtherType 0x0022）
pub mod avtp;          // 标准 AVTP 协议解析模块（IEEE 1722，EtherType 0x22F0）

use bcm::error::BcmError;
use bcm::types::ConfigPair;
use serde::Serialize;
use udp::discovery::DiscoveredDevice;
use onvif::error::OnvifError;
use onvif::{OnvifClient, OnvifDeviceInfo, OnvifCapabilities};

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

// ============================================================
// ONVIF 模块 Tauri 命令
// ============================================================

/// ONVIF 操作结果包装
#[derive(Serialize)]
pub struct OnvifResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> OnvifResult<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    fn err(e: OnvifError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }
    }
}

/// 设备发现（WS-Discovery）
///
/// 前端调用示例：
/// ```typescript
/// import { invoke } from '@tauri-apps/api';
/// const devices = await invoke('discover_devices', { timeoutMs: 5000 });
/// ```
#[tauri::command]
async fn discover_devices(timeout_ms: u64) -> OnvifResult<Vec<DiscoveredDevice>> {
    match udp::discovery::discover(timeout_ms).await {
        Ok(devices) => OnvifResult::ok(devices),
        Err(e) => OnvifResult::err(e),
    }
}

/// 获取设备基本信息
///
/// 前端调用示例：
/// ```typescript
/// const info = await invoke('get_device_info', {
///     deviceUri: 'http://192.168.1.100/onvif/device_service',
///     username: 'admin',
///     password: '12345',
/// });
/// ```
#[tauri::command]
async fn get_device_info(
    device_uri: String,
    username: Option<String>,
    password: Option<String>,
) -> OnvifResult<OnvifDeviceInfo> {
    // OnvifClient::connect() 是同步方法，直接调用
    let client = match OnvifClient::connect(
        &device_uri,
        username.as_deref(),
        password.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => return OnvifResult::err(e),
    };

    match client.get_device_info().await {
        Ok(info) => OnvifResult::ok(info),
        Err(e) => OnvifResult::err(e),
    }
}

/// 获取设备能力
///
/// 前端调用示例：
/// ```typescript
/// const caps = await invoke('get_capabilities', {
///     deviceUri: 'http://192.168.1.100/onvif/device_service',
///     username: 'admin',
///     password: '12345',
/// });
/// ```
#[tauri::command]
async fn get_capabilities(
    device_uri: String,
    username: Option<String>,
    password: Option<String>,
) -> OnvifResult<OnvifCapabilities> {
    // OnvifClient::connect() 是同步方法，直接调用
    let client = match OnvifClient::connect(
        &device_uri,
        username.as_deref(),
        password.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => return OnvifResult::err(e),
    };

    match client.get_capabilities().await {
        Ok(caps) => OnvifResult::ok(caps),
        Err(e) => OnvifResult::err(e),
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
            // ONVIF 模块命令
            discover_devices,
            get_device_info,
            get_capabilities,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
