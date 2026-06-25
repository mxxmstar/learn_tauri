//! 串口模块错误类型定义
//!
//! 使用 thiserror 库定义结构化的错误类型，提供清晰的错误上下文。
//! 设计风格与 telnet 模块的 `TelnetError` 保持一致。

use thiserror::Error;

/// 串口操作错误类型
///
/// 封装串口模块所有可能的错误，提供友好的错误信息。
/// 使用 `thiserror` 派生宏简化错误类型的实现。
#[derive(Error, Debug)]
pub enum SerialError {
    /// 端口打开失败
    ///
    /// 通常由以下原因引起：
    /// - 端口不存在（如 Windows 上没有 COM3）
    /// - 端口被其他程序占用
    /// - 权限不足（Linux/macOS 需要 dialout 组权限）
    #[error("打开串口失败: {0}")]
    OpenError(String),

    /// 端口已关闭
    ///
    /// 尝试在串口关闭后执行读写操作时返回此错误。
    #[error("串口已关闭")]
    PortClosed,

    /// 未连接
    ///
    /// 尝试在未打开串口的情况下执行操作时返回此错误。
    #[error("未连接: {0}")]
    NotConnected(String),

    /// 配置错误
    ///
    /// 配置参数无效时返回此错误，如波特率不支持、数据位无效等。
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// IO 错误
    ///
    /// 底层 IO 操作失败，如读取超时、写入失败等。
    #[error("IO 错误: {0}")]
    IoError(String),

    /// 超时错误
    ///
    /// 操作超时时返回此错误，如连接超时、读取超时等。
    #[error("操作超时: {0}")]
    Timeout(String),

    /// 协议解析错误
    ///
    /// 数据帧解析失败时返回此错误，如帧格式错误、校验失败等。
    #[error("协议解析错误: {0}")]
    ProtocolError(String),

    /// 内部错误
    ///
    /// 其他未分类的错误。
    #[error("内部错误: {0}")]
    Internal(String),
}

/// 将 serialport::Error 转换为 SerialError
///
/// 提供从 serialport 库错误到模块错误类型的无缝转换。
impl From<serialport::Error> for SerialError {
    fn from(err: serialport::Error) -> Self {
        // 将错误转换为字符串，然后根据字符串内容进行匹配
        let err_str = format!("{}", err);
        
        // 根据错误字符串进行分类
        if err_str.contains("Permission denied") || err_str.contains("权限不足") {
            SerialError::OpenError(format!("权限不足，无法打开串口: {}", err_str))
        } else if err_str.contains("Device or resource busy") || err_str.contains("正在使用") {
            SerialError::OpenError(format!("串口被占用: {}", err_str))
        } else if err_str.contains("No such file") || err_str.contains("系统找不到指定的文件") {
            SerialError::OpenError(format!("串口不存在: {}", err_str))
        } else {
            SerialError::OpenError(err_str)
        }
    }
}

/// 将 std::io::Error 转换为 SerialError
///
/// 提供从标准库 IO 错误到模块错误类型的转换。
impl From<std::io::Error> for SerialError {
    fn from(err: std::io::Error) -> Self {
        // 根据错误类型进行分类
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                SerialError::OpenError(format!("串口不存在: {}", err))
            }
            std::io::ErrorKind::PermissionDenied => {
                SerialError::OpenError(format!("权限不足: {}", err))
            }
            std::io::ErrorKind::TimedOut => {
                SerialError::Timeout(format!("IO 超时: {}", err))
            }
            _ => SerialError::IoError(format!("{}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SerialError::OpenError("COM3 不存在".to_string());
        assert_eq!(err.to_string(), "打开串口失败: COM3 不存在");

        let err = SerialError::PortClosed;
        assert_eq!(err.to_string(), "串口已关闭");

        let err = SerialError::NotConnected("请先打开串口".to_string());
        assert_eq!(err.to_string(), "未连接: 请先打开串口");

        let err = SerialError::ConfigError("波特率无效".to_string());
        assert_eq!(err.to_string(), "配置错误: 波特率无效");

        let err = SerialError::Timeout("读取超时".to_string());
        assert_eq!(err.to_string(), "操作超时: 读取超时");

        let err = SerialError::ProtocolError("帧校验失败".to_string());
        assert_eq!(err.to_string(), "协议解析错误: 帧校验失败");
    }
}
