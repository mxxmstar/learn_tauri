//! bcm 模块错误类型
//!
//! 将对 C 层的 BCM_ERR_xxx 错误码转换为 Rust 友好的错误类型。

use std::fmt;

/// bcm 操作中可能发生的错误
#[derive(Debug, Clone)]
pub enum BcmError {
    /// RPC 连接失败（含原因）
    ConnectionFailed(String),
    /// 操作超时
    Timeout,
    /// 无效的参数
    InvalidParams,
    /// 内存不足
    NoMemory,
    /// 设备不存在或已断开
    NoDevice,
    /// 权限不足
    NoPermission,
    /// RPC 调用失败（含错误码）
    RpcFailed(i32),
    /// 无效的幻数（数据损坏或协议不匹配）
    InvalidMagic,
    /// IO 错误（含原因）
    IoError(String),
}

impl fmt::Display for BcmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BcmError::ConnectionFailed(msg) => write!(f, "连接失败: {}", msg),
            BcmError::Timeout => write!(f, "操作超时"),
            BcmError::InvalidParams => write!(f, "无效参数"),
            BcmError::NoMemory => write!(f, "内存不足"),
            BcmError::NoDevice => write!(f, "设备不可用"),
            BcmError::NoPermission => write!(f, "权限不足"),
            BcmError::RpcFailed(code) => write!(f, "RPC 调用失败，错误码: 0x{:X}", code),
            BcmError::InvalidMagic => write!(f, "无效的 Magic 数字"),
            BcmError::IoError(msg) => write!(f, "IO 错误: {}", msg),
        }
    }
}

impl std::error::Error for BcmError {}

/// 将 C 层的 BCM_ERR_xxx 错误码转换为 BcmError
pub fn from_bcm_err(ret: i32) -> BcmError {
    match ret {
        super::types::BCM_ERR_TIME_OUT => BcmError::Timeout,
        super::types::BCM_ERR_INVAL_PARAMS => BcmError::InvalidParams,
        super::types::BCM_ERR_NOMEM => BcmError::NoMemory,
        super::types::BCM_ERR_NODEV => BcmError::NoDevice,
        super::types::BCM_ERR_NOPERM => BcmError::NoPermission,
        super::types::BCM_ERR_INVAL_MAGIC => BcmError::InvalidMagic,
        _ => BcmError::RpcFailed(ret),
    }
}
