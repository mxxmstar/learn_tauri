//! 高阶业务门面
//!
//! 对外暴露 async API，内部通过 tokio::task::spawn_blocking 调用
//! 阻塞的 C FFI 函数，实现异步化。同时对固件升级等复杂流程
//! 进行编排（如先启动文件服务器，再通过 RPC 通知设备下载）。

use std::net::Ipv4Addr;

use super::config;
use super::dmon;
use super::error::BcmError;
use super::file_server::FileTransferServer;
use super::rpc::RpcConnection;
use super::types::ConfigPair;
use super::update::{self, InstallConfig};

/// 将主机名字符串转为网络字节序的 u32 IP 地址
fn hostname_to_ip(host: &str) -> Result<u32, BcmError> {
    let ip: Ipv4Addr = host
        .parse()
        .map_err(|_| BcmError::InvalidParams)?;
    Ok(u32::from(ip))
}

/// 重启设备
pub async fn reboot(device_ip: &str) -> Result<(), BcmError> {
    let device_ip = device_ip.to_string();
    tokio::task::spawn_blocking(move || {
        let rpc = RpcConnection::open(&device_ip)?;
        let result = dmon::reboot(rpc.handle());
        rpc.close();
        result
    })
    .await
    .map_err(|e| BcmError::IoError(e.to_string()))?
}

/// 获取设备固件版本号
pub async fn get_version(device_ip: &str) -> Result<String, BcmError> {
    let device_ip = device_ip.to_string();
    tokio::task::spawn_blocking(move || {
        let rpc = RpcConnection::open(&device_ip)?;
        let result = dmon::get_sw_version(rpc.handle());
        rpc.close();
        result
    })
    .await
    .map_err(|e| BcmError::IoError(e.to_string()))?
}

/// 读取设备全部配置
pub async fn read_config(device_ip: &str) -> Result<Vec<ConfigPair>, BcmError> {
    let device_ip = device_ip.to_string();
    tokio::task::spawn_blocking(move || {
        let rpc = RpcConnection::open(&device_ip)?;
        let msg = config::read_raw(rpc.handle())?;
        rpc.close();
        Ok(config::parse_all(&msg))
    })
    .await
    .map_err(|e| BcmError::IoError(e.to_string()))?
}

/// 对指定分区进行健康检查
pub async fn health_check(device_ip: &str, pid: u16) -> Result<String, BcmError> {
    let device_ip = device_ip.to_string();
    tokio::task::spawn_blocking(move || {
        let rpc = RpcConnection::open(&device_ip)?;
        let version = update::health_check(&rpc, pid)?;
        rpc.close();
        Ok(format!(
            "magic=0x{:08X}, major={}, minor={}",
            version.magic, version.major, version.minor
        ))
    })
    .await
    .map_err(|e| BcmError::IoError(e.to_string()))?
}

/// 执行固件完整升级
///
/// 流程：
/// 1. 在本机启动 TCP 文件服务器
/// 2. 通过 RPC 通知设备连接文件服务器下载固件
/// 3. 设备安装完成后自动重启
pub async fn full_install(
    file_name: &str,
    file_bytes: Vec<u8>,
    device_ip: &str,
    host_ip: &str,
) -> Result<(), BcmError> {
    let device_ip = device_ip.to_string();
    let host_ip = host_ip.to_string();
    let file_name = file_name.to_string();

    let ip_addr = hostname_to_ip(&host_ip)?;

    // 启动 TCP 文件服务器传输固件
    let (server, _progress_rx) = FileTransferServer::start(file_bytes.clone())
        .await
        .map_err(|e| BcmError::IoError(e.to_string()))?;

    let port = server.port;
    let file_size = file_bytes.len() as u32;

    let install_cfg = InstallConfig::new(&file_name, file_size, ip_addr, port.into());

    // 通过 RPC 通知设备下载并安装固件
    tokio::task::spawn_blocking(move || {
        let rpc = RpcConnection::open(&device_ip)?;
        let result = update::full_install(&rpc, &install_cfg);

        if result.is_ok() {
            // 安装成功后重启设备
            let _ = dmon::reboot(rpc.handle());
            rpc.close();
            Ok(())
        } else {
            rpc.close();
            Err(result.unwrap_err())
        }
    })
    .await
    .map_err(|e| BcmError::IoError(e.to_string()))??;

    server.wait_completion().await;

    Ok(())
}
