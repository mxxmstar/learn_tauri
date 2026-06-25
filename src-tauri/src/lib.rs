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
// pub mod pcap;   // pcap 网络数据包捕获模块（网卡枚举、实时抓包）-- 暂时注释，需要 wpcap.lib
pub mod stonkam_avtp;  // Stonkam 自定义 AVTP 协议解析模块（EtherType 0x0022）
pub mod avtp;          // 标准 AVTP 协议解析模块（IEEE 1722，EtherType 0x22F0）
pub mod firmware;       // 固件处理模块（文件解密、ZIP 解压）
pub mod telnet;        // Telnet 模块（设备连接、命令执行、文件下载）
pub mod serial;        // 串口通信模块（跨平台串口通信、协议解析）

use bcm::error::BcmError;
use bcm::types::ConfigPair;
use serde::Serialize;
use udp::discovery::DiscoveredDevice;
use onvif::error::OnvifError;
use onvif::{OnvifClient, OnvifDeviceInfo, OnvifCapabilities};
use tauri::Emitter;

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

// ============================================================
// Telnet 模块 Tauri 命令
// ============================================================

/// 全局 Telnet 客户端状态
static TELNET_CLIENT: std::sync::OnceLock<tokio::sync::Mutex<Option<telnet::TelnetClient>>> =
    std::sync::OnceLock::new();

/// 获取全局客户端
fn get_telnet_client() -> &'static tokio::sync::Mutex<Option<telnet::TelnetClient>> {
    TELNET_CLIENT.get_or_init(|| tokio::sync::Mutex::new(None))
}

/// Telnet 操作结果包装
#[derive(serde::Serialize)]
struct TelnetCmdResult<T: serde::Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: serde::Serialize> TelnetCmdResult<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// 连接设备
///
/// 前端调用示例：
/// ```typescript
/// import { invoke } from '@tauri-apps/api';
/// const result = await invoke('telnet_connect', {
///     config: {
///         addr: '192.168.1.1:23',
///         connectTimeoutMs: 10000,
///         loginTimeoutMs: 15000,
///         commandTimeoutMs: 30000,
///     }
/// });
/// ```
#[tauri::command]
async fn telnet_connect(config: telnet::TelnetConfig) -> TelnetCmdResult<()> {
    let client = match telnet::TelnetClient::new(config) {
        Ok(c) => c,
        Err(e) => return TelnetCmdResult::err(&e.to_string()),
    };

    match client.connect().await {
        Ok(_) => {
            // 保存客户端到全局状态
            let mut global = get_telnet_client().lock().await;
            *global = Some(client);
            TelnetCmdResult::ok(())
        }
        Err(e) => TelnetCmdResult::err(&e.to_string()),
    }
}

/// 登录设备
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('telnet_login', {
///     username: 'root',
///     password: 'password'
/// });
/// ```
#[tauri::command]
async fn telnet_login(
    username: String,
    password: String,
) -> TelnetCmdResult<telnet::LoginResult> {
    let global = get_telnet_client().lock().await;
    let client = match global.as_ref() {
        Some(c) => c,
        None => return TelnetCmdResult::err("未连接设备，请先调用 telnet_connect"),
    };

    match client.login(&username, &password).await {
        Ok(result) => TelnetCmdResult::ok(result),
        Err(e) => TelnetCmdResult::err(&e.to_string()),
    }
}

/// 执行命令
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('telnet_send_command', {
///     command: 'ls -la'
/// });
/// ```
#[tauri::command]
async fn telnet_send_command(
    command: String,
) -> TelnetCmdResult<telnet::CommandResult> {
    let global = get_telnet_client().lock().await;
    let client = match global.as_ref() {
        Some(c) => c,
        None => return TelnetCmdResult::err("未连接设备，请先调用 telnet_connect"),
    };

    match client.execute_command(&command).await {
        Ok(result) => TelnetCmdResult::ok(result),
        Err(e) => TelnetCmdResult::err(&e.to_string()),
    }
}

/// 下载文件
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('telnet_download_file', {
///     remotePath: '/etc/config',
///     localPath: 'C:\\Users\\Downloads\\config.txt'
/// });
/// ```
///
/// 下载进度会通过事件 'telnet-download-progress' 发送到前端
#[tauri::command]
async fn telnet_download_file(
    window: tauri::Window,
    remote_path: String,
    local_path: String,
) -> TelnetCmdResult<telnet::FileDownloadResult> {
    use crate::telnet::types::DownloadProgress;

    let global = get_telnet_client().lock().await;
    let client = match global.as_ref() {
        Some(c) => c,
        None => return TelnetCmdResult::err("未连接设备，请先调用 telnet_connect"),
    };

    // 创建进度回调函数，通过事件发送到前端
    let progress_callback = Box::new(move |progress: DownloadProgress| {
        let _ = window.emit("telnet-download-progress", &progress);
    });

    match client.download_file(&remote_path, &local_path, Some(progress_callback)).await {
        Ok(result) => TelnetCmdResult::ok(result),
        Err(e) => TelnetCmdResult::err(&e.to_string()),
    }
}

/// 断开连接
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('telnet_disconnect');
/// ```
#[tauri::command]
async fn telnet_disconnect() -> TelnetCmdResult<()> {
    let mut global = get_telnet_client().lock().await;
    if let Some(client) = global.take() {
        match client.disconnect().await {
            Ok(_) => TelnetCmdResult::ok(()),
            Err(e) => TelnetCmdResult::err(&e.to_string()),
        }
    } else {
        TelnetCmdResult::ok(())
    }
}

/// 获取连接状态
///
/// 前端调用示例：
/// ```typescript
/// const status = await invoke('telnet_get_status');
/// ```
#[tauri::command]
async fn telnet_get_status() -> TelnetCmdResult<telnet::ConnectionStatus> {
    let global = get_telnet_client().lock().await;
    if let Some(client) = global.as_ref() {
        let status = client.get_status().await;
        TelnetCmdResult::ok(status)
    } else {
        TelnetCmdResult::ok(telnet::ConnectionStatus::Disconnected)
    }
}

// ============================================================
// 串口模块 Tauri 命令
// ============================================================

/// 全局串口客户端状态
static SERIAL_CLIENT: std::sync::OnceLock<tokio::sync::Mutex<Option<serial::SerialClient>>> =
    std::sync::OnceLock::new();

/// 获取全局串口客户端
fn get_serial_client() -> &'static tokio::sync::Mutex<Option<serial::SerialClient>> {
    SERIAL_CLIENT.get_or_init(|| tokio::sync::Mutex::new(None))
}

/// 串口操作结果包装
#[derive(serde::Serialize)]
struct SerialCmdResult<T: serde::Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: serde::Serialize> SerialCmdResult<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// 列出所有可用的串口
///
/// 前端调用示例：
/// ```typescript
/// import { invoke } from '@tauri-apps/api';
/// const ports = await invoke('serial_list_ports');
/// ```
#[tauri::command]
async fn serial_list_ports() -> SerialCmdResult<Vec<String>> {
    match serial::SerialClient::list_ports() {
        Ok(ports) => SerialCmdResult::ok(ports),
        Err(e) => SerialCmdResult::err(&e.to_string()),
    }
}

/// 打开串口
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('serial_open', {
///     config: {
///         portName: 'COM1',
///         baudRate: 115200,
///         dataBits: 8,
///         stopBits: 1,
///         parity: 'None',
///         flowControl: 'None',
///         timeoutMs: 1000,
///     }
/// });
/// ```
#[tauri::command]
async fn serial_open(config: serial::SerialConfig) -> SerialCmdResult<()> {
    let client = match serial::SerialClient::new(config) {
        Ok(c) => c,
        Err(e) => return SerialCmdResult::err(&e.to_string()),
    };

    match client.open().await {
        Ok(_) => {
            // 保存客户端到全局状态
            let mut global = get_serial_client().lock().await;
            *global = Some(client);
            SerialCmdResult::ok(())
        }
        Err(e) => SerialCmdResult::err(&e.to_string()),
    }
}

/// 关闭串口
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('serial_close');
/// ```
#[tauri::command]
async fn serial_close() -> SerialCmdResult<()> {
    let mut global = get_serial_client().lock().await;
    if let Some(client) = global.take() {
        match client.close().await {
            Ok(_) => SerialCmdResult::ok(()),
            Err(e) => SerialCmdResult::err(&e.to_string()),
        }
    } else {
        SerialCmdResult::ok(())
    }
}

/// 写入数据
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('serial_write', {
///     data: Array.from(new TextEncoder().encode('hello'))
/// });
/// ```
#[tauri::command]
async fn serial_write(data: Vec<u8>) -> SerialCmdResult<usize> {
    let global = get_serial_client().lock().await;
    let client = match global.as_ref() {
        Some(c) => c,
        None => return SerialCmdResult::err("串口未打开，请先调用 serial_open"),
    };

    match client.write(&data).await {
        Ok(n) => SerialCmdResult::ok(n),
        Err(e) => SerialCmdResult::err(&e.to_string()),
    }
}

/// 读取数据
///
/// 前端调用示例：
/// ```typescript
/// const result = await invoke('serial_read', { maxBytes: 1024 });
/// ```
#[tauri::command]
async fn serial_read(max_bytes: usize) -> SerialCmdResult<Vec<u8>> {
    let global = get_serial_client().lock().await;
    let client = match global.as_ref() {
        Some(c) => c,
        None => return SerialCmdResult::err("串口未打开，请先调用 serial_open"),
    };

    match client.read(max_bytes).await {
        Ok(data) => SerialCmdResult::ok(data),
        Err(e) => SerialCmdResult::err(&e.to_string()),
    }
}

/// 获取连接状态
///
/// 前端调用示例：
/// ```typescript
/// const status = await invoke('serial_get_status');
/// ```
#[tauri::command]
async fn serial_get_status() -> SerialCmdResult<serial::ConnectionStatus> {
    let global = get_serial_client().lock().await;
    if let Some(client) = global.as_ref() {
        let status = client.get_status().await;
        SerialCmdResult::ok(status)
    } else {
        SerialCmdResult::ok(serial::ConnectionStatus::Disconnected)
    }
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
            // Telnet 模块命令
            telnet_connect,
            telnet_login,
            telnet_send_command,
            telnet_download_file,
            telnet_disconnect,
            telnet_get_status,
            // 串口模块命令
            serial_list_ports,
            serial_open,
            serial_close,
            serial_write,
            serial_read,
            serial_get_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
