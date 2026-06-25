//! 串口配置管理
//!
//! 定义 `SerialConfig` 结构体，使用 builder pattern 进行配置。
//! 支持串口通信的所有常见参数配置。
//! 设计风格与 telnet 模块的 `TelnetConfig` 保持一致。

use crate::serial::types::*;
use serde::{Deserialize, Serialize};

/// 串口配置
///
/// 封装串口通信所需的所有配置参数。
/// 使用 builder pattern 构建，支持链式调用。
///
/// # 示例
///
/// ```rust
/// use crate::serial::config::SerialConfig;
///
/// let config = SerialConfig::new("COM1")
///     .baud_rate(115200)
///     .data_bits(DataBits::Eight)
///     .stop_bits(StopBits::One)
///     .parity(Parity::None)
///     .timeout_ms(1000);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    /// 串口名称
    ///
    /// - Windows: "COM1", "COM2", ...
    /// - Linux: "/dev/ttyUSB0", "/dev/ttyACM0", ...
    /// - macOS: "/dev/tty.usbserial", "/dev/tty.usbmodem", ...
    pub port_name: String,

    /// 波特率
    ///
    /// 常见值：9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600
    /// 默认：115200
    pub baud_rate: u32,

    /// 数据位
    ///
    /// 常见值：DataBits::Five, Six, Seven, Eight
    /// 默认：DataBits::Eight
    pub data_bits: DataBits,

    /// 停止位
    ///
    /// 常见值：StopBits::One, Two
    /// 默认：StopBits::One
    pub stop_bits: StopBits,

    /// 校验位
    ///
    /// 常见值：Parity::None, Odd, Even
    /// 默认：Parity::None
    pub parity: Parity,

    /// 流控制
    ///
    /// 常见值：FlowControl::None, Software, Hardware
    /// 默认：FlowControl::None
    pub flow_control: FlowControl,

    /// 读取超时（毫秒）
    ///
    /// 设置读取操作的超时时间。
    /// 默认：1000 ms
    pub timeout_ms: u64,

    /// 读取缓冲区大小（字节）
    ///
    /// 设置底层读取缓冲区的大小。
    /// 默认：4096 字节
    pub read_buffer_size: usize,
}

impl SerialConfig {
    /// 创建新的串口配置
    ///
    /// # 参数
    /// * `port_name` - 串口名称（如 "COM1" 或 "/dev/ttyUSB0"）
    ///
    /// # 返回值
    /// 返回使用默认值的 SerialConfig 实例
    pub fn new(port_name: &str) -> Self {
        Self {
            port_name: port_name.to_string(),
            baud_rate: 115200,           // 默认波特率
            data_bits: DataBits::default(), // 默认 8 数据位
            stop_bits: StopBits::default(), // 默认 1 停止位
            parity: Parity::default(),    // 默认无校验
            flow_control: FlowControl::default(), // 默认无流控制
            timeout_ms: 1000,            // 默认 1 秒超时
            read_buffer_size: 4096,       // 默认 4KB 缓冲区
        }
    }

    /// 设置波特率（链式调用）
    ///
    /// # 参数
    /// * `baud_rate` - 波特率（如 9600, 115200）
    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    /// 设置数据位（链式调用）
    ///
    /// # 参数
    /// * `data_bits` - 数据位配置
    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    /// 设置停止位（链式调用）
    ///
    /// # 参数
    /// * `stop_bits` - 停止位配置
    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    /// 设置校验位（链式调用）
    ///
    /// # 参数
    /// * `parity` - 校验位配置
    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    /// 设置流控制（链式调用）
    ///
    /// # 参数
    /// * `flow_control` - 流控制配置
    pub fn flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    /// 设置读取超时（链式调用）
    ///
    /// # 参数
    /// * `timeout_ms` - 超时时间（毫秒）
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// 设置读取缓冲区大小（链式调用）
    ///
    /// # 参数
    /// * `size` - 缓冲区大小（字节）
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.read_buffer_size = size;
        self
    }

    /// 验证配置有效性
    ///
    /// 检查配置参数是否在有效范围内。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()，失败时返回错误信息
    pub fn validate(&self) -> Result<(), String> {
        // 验证波特率
        // 常见波特率列表
        let valid_baud_rates = [
            110, 300, 600, 1200, 2400, 4800, 9600,
            14400, 19200, 28800, 38400, 56000, 57600,
            115200, 128000, 153600, 230400, 256000,
            460800, 500000, 576000, 921600, 1000000,
            1152000, 1500000, 2000000, 2500000, 3000000,
        ];
        if !valid_baud_rates.contains(&self.baud_rate) {
            // 警告但不强制失败，因为某些设备支持非标准波特率
            eprintln!("警告: 波特率 {} 不是标准值", self.baud_rate);
        }

        // 验证端口名称
        if self.port_name.is_empty() {
            return Err("端口名称不能为空".to_string());
        }

        // 验证超时时间
        if self.timeout_ms == 0 {
            return Err("超时时间必须大于 0".to_string());
        }

        // 验证缓冲区大小
        if self.read_buffer_size < 64 {
            return Err("缓冲区大小必须至少为 64 字节".to_string());
        }

        Ok(())
    }

    /// 转换为 serialport 库的 PortSettings
    ///
    /// 将配置转换为 serialport 库所需的格式。
    pub fn to_port_settings(&self) -> (u32, serialport::DataBits, serialport::StopBits, serialport::Parity, serialport::FlowControl) {
        (
            self.baud_rate,
            self.data_bits.into(),
            self.stop_bits.into(),
            self.parity.into(),
            self.flow_control.into(),
        )
    }
}

impl Default for SerialConfig {
    /// 默认配置
    ///
    /// 使用 COM1 端口（Windows）和常用参数。
    /// 注意：在实际使用中应根据目标平台调整端口名称。
    fn default() -> Self {
        // 根据平台选择默认端口名称
        let port_name = if cfg!(windows) {
            "COM1".to_string()
        } else if cfg!(target_os = "macos") {
            "/dev/tty.usbserial".to_string()
        } else {
            "/dev/ttyUSB0".to_string()
        };

        Self::new(&port_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = SerialConfig::new("COM1");
        assert_eq!(config.port_name, "COM1");
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.stop_bits, StopBits::One);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.flow_control, FlowControl::None);
        assert_eq!(config.timeout_ms, 1000);
        assert_eq!(config.read_buffer_size, 4096);
    }

    #[test]
    fn test_config_builder() {
        let config = SerialConfig::new("COM3")
            .baud_rate(9600)
            .data_bits(DataBits::Seven)
            .stop_bits(StopBits::Two)
            .parity(Parity::Even)
            .flow_control(FlowControl::Hardware)
            .timeout_ms(500)
            .read_buffer_size(2048);

        assert_eq!(config.port_name, "COM3");
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.data_bits, DataBits::Seven);
        assert_eq!(config.stop_bits, StopBits::Two);
        assert_eq!(config.parity, Parity::Even);
        assert_eq!(config.flow_control, FlowControl::Hardware);
        assert_eq!(config.timeout_ms, 500);
        assert_eq!(config.read_buffer_size, 2048);
    }

    #[test]
    fn test_config_validate() {
        let config = SerialConfig::new("COM1");
        assert!(config.validate().is_ok());

        let config = SerialConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_default() {
        let config = SerialConfig::default();
        assert_eq!(config.baud_rate, 115200);
    }
}
