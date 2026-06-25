//! 串口模块数据类型定义
//!
//! 定义串口连接状态、操作结果等核心数据类型。
//! 设计风格与 telnet 模块保持一致。

use serde::{Deserialize, Serialize};

/// 串口连接状态
///
/// 表示串口客户端的当前连接状态，用于状态管理和 UI 显示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// 已断开连接
    Disconnected,
    /// 正在连接中
    Connecting,
    /// 已连接（串口已打开）
    Connected,
    /// 连接失败
    Failed(String),
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "已断开"),
            ConnectionStatus::Connecting => write!(f, "连接中"),
            ConnectionStatus::Connected => write!(f, "已连接"),
            ConnectionStatus::Failed(msg) => write!(f, "连接失败: {}", msg),
        }
    }
}

/// 串口操作结果包装
///
/// 标准化的操作结果类型，统一模块内所有操作的返回格式。
/// 参考 telnet 模块的 `TelnetOpResult<T>` 设计。
pub type SerialOpResult<T> = Result<T, crate::serial::error::SerialError>;

/// 数据位配置
///
/// 串口通信的数据位设置，常见值为 8。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBits {
    /// 5 数据位
    Five = 5,
    /// 6 数据位
    Six = 6,
    /// 7 数据位
    Seven = 7,
    /// 8 数据位
    Eight = 8,
}

impl Default for DataBits {
    fn default() -> Self {
        DataBits::Eight
    }
}

impl From<DataBits> for serialport::DataBits {
    fn from(bits: DataBits) -> Self {
        match bits {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        }
    }
}

/// 停止位配置
///
/// 串口通信的停止位设置，常见值为 1。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBits {
    /// 1 停止位
    One = 1,
    /// 2 停止位
    Two = 2,
}

impl Default for StopBits {
    fn default() -> Self {
        StopBits::One
    }
}

impl From<StopBits> for serialport::StopBits {
    fn from(bits: StopBits) -> Self {
        match bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

/// 校验位配置
///
/// 串口通信的校验位设置，用于错误检测。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    /// 无校验（默认）
    None,
    /// 奇校验
    Odd,
    /// 偶校验
    Even,
}

impl Default for Parity {
    fn default() -> Self {
        Parity::None
    }
}

impl From<Parity> for serialport::Parity {
    fn from(parity: Parity) -> Self {
        match parity {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

/// 流控制配置
///
/// 串口通信的硬件/软件流控制设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    /// 无流控制（默认）
    None,
    /// 软件流控制（XON/XOFF）
    Software,
    /// 硬件流控制（RTS/CTS）
    Hardware,
}

impl Default for FlowControl {
    fn default() -> Self {
        FlowControl::None
    }
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(fc: FlowControl) -> Self {
        match fc {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "已断开");
        assert_eq!(ConnectionStatus::Connecting.to_string(), "连接中");
        assert_eq!(ConnectionStatus::Connected.to_string(), "已连接");
        assert_eq!(
            ConnectionStatus::Failed("超时".to_string()).to_string(),
            "连接失败: 超时"
        );
    }

    #[test]
    fn test_data_bits_default() {
        assert_eq!(DataBits::default(), DataBits::Eight);
    }

    #[test]
    fn test_stop_bits_default() {
        assert_eq!(StopBits::default(), StopBits::One);
    }

    #[test]
    fn test_parity_default() {
        assert_eq!(Parity::default(), Parity::None);
    }

    #[test]
    fn test_flow_control_default() {
        assert_eq!(FlowControl::default(), FlowControl::None);
    }
}
