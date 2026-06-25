//! 串口客户端核心实现
//!
//! 实现 `SerialClient` 结构体，封装串口连接、数据读写和协议解析功能。
//! 使用 `serialport` crate 提供跨平台的串口通信能力。
//! 设计风格与 telnet 模块的 `TelnetClient` 保持一致。
//!
//! # 同步和异步模式
//!
//! 本模块同时提供同步和异步两种使用模式：
//! - 同步模式：方法名以 `_sync` 结尾，直接调用，阻塞当前线程
//! - 异步模式：方法名不带后缀，使用 `tokio::task::spawn_blocking` 包装同步操作
//!
//! # 示例
//!
//! ```rust
//! use crate::serial::{SerialClient, SerialConfig};
//!
//! // 同步模式
//! let client = SerialClient::new(config.clone())?;
//! client.open_sync()?;
//! client.write_sync(b"hello")?;
//! let data = client.read_sync(100)?;
//! client.close_sync()?;
//!
//! // 异步模式
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = SerialClient::new(config.clone())?;
//!     client.open().await?;
//!     client.write(b"hello").await?;
//!     let data = client.read(100).await?;
//!     client.close().await?;
//!     Ok(())
//! }
//! ```

use crate::serial::config::SerialConfig;
use crate::serial::error::SerialError;
use crate::serial::protocol::{ProtocolParser, ParseResult};
use crate::serial::types::*;
use std::sync::Arc;
use std::sync::Mutex;

/// 串口客户端
///
/// 提供串口通信的核心功能，包括：
/// - 连接管理（打开/关闭串口）
/// - 数据读写（支持同步和异步操作）
/// - 协议解析（支持自定义协议扩展）
///
/// # 线程安全
///
/// 内部使用 `Arc<Mutex>` 共享状态，可以安全地克隆并在多个线程/任务间使用。
///
/// # 同步和异步模式
///
/// 本结构体同时提供同步和异步两种使用模式：
/// - 同步模式：方法名以 `_sync` 结尾（如 `open_sync()`, `write_sync()`）
/// - 异步模式：方法名不带后缀（如 `open()`, `write()`），内部使用 `spawn_blocking`
///
/// # 示例
///
/// ```rust
/// use crate::serial::{SerialClient, SerialConfig};
///
/// // 同步模式
/// let client = SerialClient::new(config.clone())?;
/// client.open_sync()?;
/// client.write_sync(b"hello")?;
/// let data = client.read_sync(100)?;
/// client.close_sync()?;
///
/// // 异步模式
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = SerialClient::new(config.clone())?;
///     client.open().await?;
///     client.write(b"hello").await?;
///     let data = client.read(100).await?;
///     client.close().await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct SerialClient {
    /// 客户端配置
    config: Arc<SerialConfig>,
    /// 串口连接（连接后存在）
    ///
    /// 使用 `Option` 表示连接的可选性
    /// 使用 `Box<dyn SerialPort>` 实现跨平台兼容
    port: Arc<Mutex<Option<Box<dyn serialport::SerialPort>>>>,
    /// 连接状态
    status: Arc<Mutex<ConnectionStatus>>,
    /// 协议解析器（可选）
    ///
    /// 如果设置了协议解析器，读写操作会自动进行协议封装/解析
    protocol_parser: Arc<Mutex<Option<Arc<dyn ProtocolParser>>>>,
}

impl SerialClient {
    /// 创建新的串口客户端实例
    ///
    /// # 参数
    /// * `config` - 串口配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 SerialClient 实例
    pub fn new(config: SerialConfig) -> SerialOpResult<Self> {
        // 验证配置有效性
        config.validate()
            .map_err(|e| SerialError::ConfigError(e))?;

        Ok(Self {
            config: Arc::new(config),
            port: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            protocol_parser: Arc::new(Mutex::new(None)),
        })
    }

    // ============================================================
    // 同步方法（阻塞当前线程）
    // ============================================================

    /// 打开串口（同步）
    ///
    /// 根据配置打开串口设备，设置波特率、数据位等参数。
    /// 此方法会阻塞当前线程，直到操作完成或超时。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub fn open_sync(&self) -> SerialOpResult<()> {
        // 更新状态为正在连接
        {
            let mut status = self.status.lock().unwrap();
            *status = ConnectionStatus::Connecting;
        } // 释放锁

        // 构建串口配置并打开
        let port_result = serialport::new(
            &self.config.port_name,
            self.config.baud_rate,
        )
        .data_bits(self.config.data_bits.into())
        .stop_bits(self.config.stop_bits.into())
        .parity(self.config.parity.into())
        .flow_control(self.config.flow_control.into())
        .timeout(std::time::Duration::from_millis(self.config.timeout_ms))
        .open();

        match port_result {
            Ok(port) => {
                // 保存连接（port 已经是 Box<dyn SerialPort>）
                let mut port_guard = self.port.lock().unwrap();
                *port_guard = Some(port);
                drop(port_guard);

                // 更新状态为已连接
                let mut status = self.status.lock().unwrap();
                *status = ConnectionStatus::Connected;

                Ok(())
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                let mut status = self.status.lock().unwrap();
                *status = ConnectionStatus::Failed(err_msg.clone());
                drop(status);

                Err(SerialError::from(e))
            }
        }
    }

    /// 关闭串口（同步）
    ///
    /// 关闭串口连接并释放资源。
    /// 此方法会阻塞当前线程，直到操作完成。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub fn close_sync(&self) -> SerialOpResult<()> {
        // 取出 port，drop 时会自动关闭连接
        let mut port_guard = self.port.lock().unwrap();
        let _ = port_guard.take();
        drop(port_guard);

        let mut status = self.status.lock().unwrap();
        *status = ConnectionStatus::Disconnected;

        Ok(())
    }

    /// 写入数据（同步）
    ///
    /// 向串口写入数据。如果设置了协议解析器，会自动调用 `encode_frame()` 封装数据。
    /// 此方法会阻塞当前线程，直到操作完成或超时。
    ///
    /// # 参数
    /// * `data` - 要写入的数据
    ///
    /// # 返回值
    /// 返回 Result，成功时返回写入的字节数
    pub fn write_sync(&self, data: &[u8]) -> SerialOpResult<usize> {
        // 检查连接状态
        if !self.is_connected_sync() {
            return Err(SerialError::NotConnected("请先打开串口".to_string()));
        }

        let mut port_guard = self.port.lock().unwrap();
        let port = port_guard
            .as_mut()
            .ok_or_else(|| SerialError::PortClosed)?;

        // 如果设置了协议解析器，封装数据
        let write_data = {
            let parser_guard = self.protocol_parser.lock().unwrap();
            if let Some(parser) = parser_guard.as_ref() {
                parser.encode_frame(data)
            } else {
                data.to_vec()
            }
        };

        // 写入数据（SerialPort 实现了 std::io::Write）
        match port.write(&write_data) {
            Ok(n) => Ok(n),
            Err(e) => Err(SerialError::IoError(format!("写入失败: {}", e))),
        }
    }

    /// 读取数据（同步）
    ///
    /// 从串口读取数据。如果设置了协议解析器，会自动解析完整帧。
    /// 此方法会阻塞当前线程，直到读取到数据、超时或发生错误。
    ///
    /// # 参数
    /// * `max_bytes` - 最大读取字节数（仅在无协议解析器时使用）
    ///
    /// # 返回值
    /// 返回 Result，成功时包含读取的数据
    pub fn read_sync(&self, max_bytes: usize) -> SerialOpResult<Vec<u8>> {
        // 检查连接状态
        if !self.is_connected_sync() {
            return Err(SerialError::NotConnected("请先打开串口".to_string()));
        }

        let mut port_guard = self.port.lock().unwrap();
        let port = port_guard
            .as_mut()
            .ok_or_else(|| SerialError::PortClosed)?;

        // 获取协议解析器（如果有）
        let parser_opt = {
            let parser_guard = self.protocol_parser.lock().unwrap();
            parser_guard.clone()
        };

        // 如果有协议解析器，使用解析器读取完整帧
        if let Some(parser) = parser_opt {
            // 读取数据到缓冲区
            let mut buffer = vec![0u8; self.config.read_buffer_size];
            let mut all_data = Vec::new();

            loop {
                match port.read(&mut buffer) {
                    Ok(0) => {
                        // 连接关闭
                        return Err(SerialError::NotConnected("连接已关闭".to_string()));
                    }
                    Ok(n) => {
                        all_data.extend_from_slice(&buffer[..n]);

                        // 尝试解析帧
                        match parser.parse_frame(&all_data) {
                            ParseResult::Complete(frame_data, consumed) => {
                                // 解析成功，返回帧数据
                                // TODO: 可以优化：保存未使用的数据到读取缓冲区
                                let _remaining = &all_data[consumed..];
                                return Ok(frame_data);
                            }
                            ParseResult::Incomplete => {
                                // 数据不完整，继续读取
                                continue;
                            }
                            ParseResult::Error(msg) => {
                                return Err(SerialError::ProtocolError(msg));
                            }
                        }
                    }
                    Err(e) => {
                        // 检查是否是超时错误
                        if e.kind() == std::io::ErrorKind::TimedOut {
                            // 超时，但没有完整帧
                            return Err(SerialError::Timeout("读取超时，未收到完整帧".to_string()));
                        }
                        return Err(SerialError::IoError(format!("读取失败: {}", e)));
                    }
                }
            }
        } else {
            // 无协议解析器，直接读取指定字节数
            let mut buffer = vec![0u8; max_bytes];
            match port.read(&mut buffer) {
                Ok(0) => {
                    Err(SerialError::NotConnected("连接已关闭".to_string()))
                }
                Ok(n) => {
                    buffer.truncate(n);
                    Ok(buffer)
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        Err(SerialError::Timeout("读取超时".to_string()))
                    } else {
                        Err(SerialError::IoError(format!("读取失败: {}", e)))
                    }
                }
            }
        }
    }

    /// 设置协议解析器（同步）
    ///
    /// 设置自定义的协议解析器，用于数据帧的封装和解析。
    ///
    /// # 参数
    /// * `parser` - 实现 `ProtocolParser` trait 的解析器
    pub fn set_protocol_parser_sync(&self, parser: Arc<dyn ProtocolParser>) {
        let mut parser_guard = self.protocol_parser.lock().unwrap();
        *parser_guard = Some(parser);
    }

    /// 清除协议解析器（同步）
    ///
    /// 清除自定义的协议解析器，恢复简单读写模式。
    pub fn clear_protocol_parser_sync(&self) {
        let mut parser_guard = self.protocol_parser.lock().unwrap();
        *parser_guard = None;
    }

    /// 是否已连接（同步）
    ///
    /// 检查串口是否已打开。
    pub fn is_connected_sync(&self) -> bool {
        let status = self.status.lock().unwrap();
        matches!(*status, ConnectionStatus::Connected)
    }

    /// 获取连接状态（同步）
    pub fn get_status_sync(&self) -> ConnectionStatus {
        let status = self.status.lock().unwrap();
        status.clone()
    }

    // ============================================================
    // 异步方法（使用 spawn_blocking 包装同步方法）
    // ============================================================

    /// 打开串口（异步）
    ///
    /// 根据配置打开串口设备，设置波特率、数据位等参数。
    /// 此方法会在后台线程中执行，不会阻塞 tokio 运行时。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn open(&self) -> SerialOpResult<()> {
        let client = self.clone();
        match tokio::task::spawn_blocking(move || client.open_sync()).await {
            Ok(result) => result,
            Err(e) => Err(SerialError::IoError(format!("后台任务执行失败: {}", e))),
        }
    }

    /// 关闭串口（异步）
    ///
    /// 关闭串口连接并释放资源。
    /// 此方法会在后台线程中执行，不会阻塞 tokio 运行时。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn close(&self) -> SerialOpResult<()> {
        let client = self.clone();
        match tokio::task::spawn_blocking(move || client.close_sync()).await {
            Ok(result) => result,
            Err(e) => Err(SerialError::IoError(format!("后台任务执行失败: {}", e))),
        }
    }

    /// 写入数据（异步）
    ///
    /// 向串口写入数据。如果设置了协议解析器，会自动调用 `encode_frame()` 封装数据。
    /// 此方法会在后台线程中执行，不会阻塞 tokio 运行时。
    ///
    /// # 参数
    /// * `data` - 要写入的数据
    ///
    /// # 返回值
    /// 返回 Result，成功时返回写入的字节数
    pub async fn write(&self, data: &[u8]) -> SerialOpResult<usize> {
        let client = self.clone();
        let data_vec = data.to_vec();
        match tokio::task::spawn_blocking(move || client.write_sync(&data_vec)).await {
            Ok(result) => result,
            Err(e) => Err(SerialError::IoError(format!("后台任务执行失败: {}", e))),
        }
    }

    /// 读取数据（异步）
    ///
    /// 从串口读取数据。如果设置了协议解析器，会自动解析完整帧。
    /// 此方法会在后台线程中执行，不会阻塞 tokio 运行时。
    ///
    /// # 参数
    /// * `max_bytes` - 最大读取字节数（仅在无协议解析器时使用）
    ///
    /// # 返回值
    /// 返回 Result，成功时包含读取的数据
    pub async fn read(&self, max_bytes: usize) -> SerialOpResult<Vec<u8>> {
        let client = self.clone();
        match tokio::task::spawn_blocking(move || client.read_sync(max_bytes)).await {
            Ok(result) => result,
            Err(e) => Err(SerialError::IoError(format!("后台任务执行失败: {}", e))),
        }
    }

    /// 设置协议解析器（异步）
    ///
    /// 设置自定义的协议解析器，用于数据帧的封装和解析。
    ///
    /// # 参数
    /// * `parser` - 实现 `ProtocolParser` trait 的解析器
    pub async fn set_protocol_parser(&self, parser: Arc<dyn ProtocolParser>) {
        let client = self.clone();
        // 使用 spawn_blocking 并在闭包中直接调用同步方法
        let _ = tokio::task::spawn_blocking(move || {
            client.set_protocol_parser_sync(parser);
        })
        .await;
    }

    /// 清除协议解析器（异步）
    ///
    /// 清除自定义的协议解析器，恢复简单读写模式。
    pub async fn clear_protocol_parser(&self) {
        let client = self.clone();
        let _ = tokio::task::spawn_blocking(move || {
            client.clear_protocol_parser_sync();
        })
        .await;
    }

    /// 是否已连接（异步）
    ///
    /// 检查串口是否已打开。
    pub async fn is_connected(&self) -> bool {
        let client = self.clone();
        match tokio::task::spawn_blocking(move || client.is_connected_sync()).await {
            Ok(result) => result,
            Err(_) => false, // 如果后台任务失败，返回 false
        }
    }

    /// 获取连接状态（异步）
    pub async fn get_status(&self) -> ConnectionStatus {
        let client = self.clone();
        match tokio::task::spawn_blocking(move || client.get_status_sync()).await {
            Ok(status) => status,
            Err(_) => ConnectionStatus::Failed("后台任务执行失败".to_string()),
        }
    }

    // ============================================================
    // 公共方法（不区分同步/异步）
    // ============================================================

    /// 获取配置
    pub fn get_config(&self) -> SerialConfig {
        (*self.config).clone()
    }

    /// 列出所有可用的串口
    ///
    /// 静态方法，用于发现系统上的可用串口。
    /// 此方法是同步的，但通常很快完成，不会阻塞太久。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含可用串口名称列表
    pub fn list_ports() -> SerialOpResult<Vec<String>> {
        match serialport::available_ports() {
            Ok(ports) => {
                let port_names = ports
                    .into_iter()
                    .map(|p| p.port_name)
                    .collect();
                Ok(port_names)
            }
            Err(e) => Err(SerialError::OpenError(format!("枚举串口失败: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial::protocol::DelimiterParser;

    #[tokio::test]
    async fn test_client_create() {
        let config = SerialConfig::new("COM1");
        let client = SerialClient::new(config);
        assert!(client.is_ok());
    }

    // 注意：实际打开串口需要硬件支持，这里只能测试创建和状态管理
    #[tokio::test]
    async fn test_client_status() {
        let config = SerialConfig::new("COM1");
        let client = SerialClient::new(config).unwrap();

        // 初始状态应该是 Disconnected
        let status = client.get_status().await;
        assert_eq!(status, ConnectionStatus::Disconnected);

        // 测试未连接时的写入
        let result = client.write(b"test").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_list_ports() {
        // 列出可用串口（不会实际打开，应该总是成功）
        let result = SerialClient::list_ports();
        assert!(result.is_ok());
        if let Ok(ports) = result {
            println!("可用串口: {:?}", ports);
        }
    }

    #[test]
    fn test_sync_methods_exist() {
        // 测试同步方法是否存在（编译时检查）
        let config = SerialConfig::new("COM1");
        let client = SerialClient::new(config).unwrap();
        
        // 测试方法调用（不会实际执行，因为串口不存在）
        let _ = client.open_sync();
        let _ = client.is_connected_sync();
        let _ = client.get_status_sync();
    }
}
