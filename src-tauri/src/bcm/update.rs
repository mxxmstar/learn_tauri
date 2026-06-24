//! UPDATE（固件更新）操作
//!
//! 在 Rust 侧完全重写 UPDATE 协议层，通过 RPC_SendRecv 实现
//! 健康检查和固件完整安装功能，无需依赖 C 代码。

use super::error::BcmError;
use super::rpc::RpcConnection;
use super::types::*;

/// 对指定分区进行健康检查，获取版本信息
///
/// # 参数
/// * `rpc` - RPC 连接
/// * `pid` - 分区 ID
///
/// # 返回值
/// * `Ok(IMGL_VersionType)` - 分区镜像版本信息
/// * `Err(BcmError)` - 健康检查失败
pub fn health_check(rpc: &RpcConnection, pid: u16) -> Result<IMGL_VersionType, BcmError> {
    let msg = UPDATE_HealthCheckMsgType {
        pid,
        _pad1: [0u8; 2],
        version: IMGL_VersionType {
            magic: 0,
            major: 0,
            minor: 0,
            build_info: [0u8; 116],
        },
    };

    let in_data = as_u8_slice(&msg);
    let mut out_data = vec![0u8; std::mem::size_of::<UPDATE_HealthCheckMsgType>()];

    rpc.send_recv(UPDATE_ID_HEALTH_CHECK, in_data, &mut out_data)?;

    let result: UPDATE_HealthCheckMsgType = unsafe { std::mem::transmute_copy(&out_data[0]) };
    Ok(result.version)
}

/// 固件安装配置
pub struct InstallConfig {
    pub nvm_channel: u32,     ///< NVM 通道
    pub fetch_channel: u32,   ///< 获取通道
    pub nvm_erase_size: u32,  ///< 擦除大小
    pub file_size: u32,       ///< 固件文件大小
    pub ip_addr: u32,         ///< 文件服务器 IP（网络字节序）
    pub port_num: u32,        ///< 文件服务器端口
    /// 文件名
    pub name: [u8; 256],
}

impl InstallConfig {
    /// 创建安装配置
    ///
    /// # 参数
    /// * `file_name` - 固件文件名
    /// * `file_size` - 固件文件大小（字节）
    /// * `ip_addr` - 本机 IP 地址（供设备连接）
    /// * `port_num` - 本机 TCP 文件服务端口
    pub fn new(file_name: &str, file_size: u32, ip_addr: u32, port_num: u32) -> Self {
        let mut name = [0u8; 256];
        let bytes = file_name.as_bytes();
        let len = bytes.len().min(255);
        name[..len].copy_from_slice(&bytes[..len]);

        InstallConfig {
            nvm_channel: 0x4E564D30,   // "NVM0"
            fetch_channel: 0x52465450, // "RFTP"（RPC FTP）
            nvm_erase_size: 0x1b0000,
            file_size,
            ip_addr,
            port_num,
            name,
        }
    }
}

/// 执行固件完整安装
///
/// 设备通过 RPC FTP 通道从本机的文件传输服务器获取固件数据。
///
/// # 参数
/// * `rpc` - RPC 连接
/// * `cfg` - 安装配置
///
/// # 返回值
/// * `Ok(u32)` - 设备实际接收的文件大小
/// * `Err(BcmError)` - 安装失败
pub fn full_install(rpc: &RpcConnection, cfg: &InstallConfig) -> Result<u32, BcmError> {
    let install_cfg = UPDATE_InstallCfgMsgType {
        nvm_channel: cfg.nvm_channel,
        fetch_channel: cfg.fetch_channel,
        reserved: [0u32; 2],
        nvm_erase_size: cfg.nvm_erase_size,
        file_size: cfg.file_size,
        ip_addr: cfg.ip_addr,
        port_num: cfg.port_num,
        info: [0u32; 4],
        name: cfg.name,
    };

    let install_msg = UPDATE_InstallMsgType {
        cfg: install_cfg,
        recv_file_size: 0,
    };

    let in_data = as_u8_slice(&install_msg);
    let mut out_data = vec![0u8; std::mem::size_of::<UPDATE_InstallMsgType>()];

    rpc.send_recv(UPDATE_ID_FULL_INSTALL, in_data, &mut out_data)?;

    let result: UPDATE_InstallMsgType = unsafe { std::mem::transmute_copy(&out_data[0]) };
    Ok(result.recv_file_size)
}

/// 将任意 Sized 类型转换为字节切片（用于 FFI 消息发送）
fn as_u8_slice<T: Sized>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}
