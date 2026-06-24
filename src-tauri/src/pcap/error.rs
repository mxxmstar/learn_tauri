//! pcap 模块错误类型定义
//!
//! 使用 `thiserror` 派生宏，为 pcap 模块中所有可能的错误场景提供
//! 清晰、可读的错误消息。错误类型遵循项目现有约定（参考 `rtp::error::RtpError`）。

use thiserror::Error;

/// pcap 模块的统一错误类型。
///
/// 覆盖网卡枚举、设备打开、捕获启动/停止等所有操作中的错误场景。
#[derive(Error, Debug)]
pub enum PcapError {
    /// 枚举网卡列表失败。
    ///
    /// 包装 `pcap::Error` 中的底层错误信息（如权限不足、npcap 未安装等）。
    #[error("枚举网卡设备失败: {0}")]
    ListDevicesError(String),

    /// 未找到指定名称的网卡设备。
    #[error("未找到网卡设备: {0}")]
    DeviceNotFound(String),

    /// 打开指定网卡设备失败。
    ///
    /// # 参数
    /// - `device`: 网卡设备名称
    /// - `reason`: 底层错误原因（来自 libpcap/Npcap）
    #[error("打开网卡 '{device}' 失败: {reason}")]
    OpenDeviceError {
        device: String,
        reason: String,
    },

    /// 捕获句柄未就绪（未调用 open 或已关闭）。
    #[error("捕获句柄未就绪，请先调用 open() 打开网卡")]
    NotReady,

    /// 捕获循环已在运行中，重复调用 start 会触发此错误。
    #[error("捕获循环已在运行中，无法重复启动")]
    AlreadyRunning,

    /// 捕获循环未运行，调用 stop 或获取数据包时触发此错误。
    #[error("捕获循环未运行")]
    NotRunning,

    /// 捕获过程中发生错误（如 `next_packet()` 返回非超时类错误）。
    #[error("捕获错误: {0}")]
    CaptureError(String),

    /// 设置抓包过滤器（BPF 表达式）失败。
    #[error("设置过滤器失败: {0}")]
    SetFilterError(String),
}
