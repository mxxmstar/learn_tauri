//! TCP 消息协议模块
//!
//! 该模块定义了 TCP 通信中使用的消息格式和编解码逻辑。
//!
//! # TCP 消息帧化（Framing）
//!
//! 与 UDP 不同，TCP 是**流式协议**，没有消息边界。
//! 如果直接发送多条消息，接收端可能一次性读到多条消息的一部分（粘包），
//! 或读到一条消息的一部分（半包）。
//!
//! 为解决此问题，本模块采用**长度前缀**帧化方案：
//!
//! ```text
//! ┌──────────────┬─────────────────────────────┐
//! │ 长度字段(4B)  │       消息内容 (N 字节)       │
//! │ (大端 u32)   │   (JSON 编码的 UdpMessage)   │
//! └──────────────┴─────────────────────────────┘
//! ```
//!
//! 实际的帧化编解码由 `tokio_util::codec::LengthDelimitedCodec` 完成，
//! 本模块负责将 JSON 消息序列化为字节，再交给 codec 处理帧边界。
//!
//! # 消息类型
//!
//! 消息使用与 UDP 模块相同的 JSON 结构，但通过帧化传输保证消息完整性。

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 获取当前 Unix 时间戳（毫秒）
fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// TCP 消息类型
///
/// 定义不同类型的 TCP 消息，用于区分消息用途。
/// 与 UDP 模块的 MessageType 保持一致，便于互通。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// 普通数据消息
    Data,
    /// 广播消息（服务端转发给所有连接）
    Broadcast,
    /// 心跳 Ping
    Ping,
    /// 心跳 Pong
    Pong,
    /// 自定义消息类型
    Custom(String),
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Data => write!(f, "Data"),
            MessageType::Broadcast => write!(f, "Broadcast"),
            MessageType::Ping => write!(f, "Ping"),
            MessageType::Pong => write!(f, "Pong"),
            MessageType::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

/// TCP 结构化消息
///
/// 使用 JSON 编码的消息格式，配合长度前缀帧化传输。
///
/// # 序列化示例
///
/// ```json
/// {
///     "msg_type": "Data",
///     "payload": "Hello, TCP!",
///     "timestamp": 1719000000000
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpMessage {
    /// 消息类型
    pub msg_type: MessageType,
    /// 消息负载（文本内容）
    pub payload: String,
    /// 时间戳（Unix 毫秒）
    pub timestamp: u64,
}

impl TcpMessage {
    /// 创建新的数据消息
    ///
    /// # 参数
    /// * `payload` - 消息内容
    pub fn data(payload: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Data,
            payload: payload.into(),
            timestamp: unix_millis(),
        }
    }

    /// 创建新的广播消息
    pub fn broadcast(payload: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Broadcast,
            payload: payload.into(),
            timestamp: unix_millis(),
        }
    }

    /// 创建 Ping 消息
    pub fn ping() -> Self {
        Self {
            msg_type: MessageType::Ping,
            payload: String::new(),
            timestamp: unix_millis(),
        }
    }

    /// 创建 Pong 消息（用于回复 Ping）
    pub fn pong(original_timestamp: u64) -> Self {
        Self {
            msg_type: MessageType::Pong,
            payload: original_timestamp.to_string(),
            timestamp: unix_millis(),
        }
    }

    /// 创建自定义类型消息
    pub fn custom(msg_type: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Custom(msg_type.into()),
            payload: payload.into(),
            timestamp: unix_millis(),
        }
    }

    /// 将消息编码为字节序列（JSON 格式）
    ///
    /// 注意：此方法仅完成 JSON 序列化，不含长度前缀。
    /// 长度前缀的添加由 `tokio_util::codec::LengthDelimitedCodec` 自动完成。
    ///
    /// # 返回值
    /// 返回 Result，成功时包含编码后的字节向量
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("消息编码失败: {}", e))
    }

    /// 从字节序列解码消息（JSON 格式）
    ///
    /// 注意：传入的 `data` 应为去掉长度前缀后的纯消息体，
    /// 长度前缀的剥离由 codec 自动完成。
    ///
    /// # 参数
    /// * `data` - 消息体字节序列
    ///
    /// # 返回值
    /// 返回 Result，成功时包含解码后的消息
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| format!("消息解码失败: {}", e))
    }

    /// 从任意字节缓冲解码消息
    ///
    /// 配合 `tokio_util::codec::LengthDelimitedCodec` 使用，
    /// codec 输出的帧类型为 `BytesMut`（读取端）或 `Bytes`。
    /// 本方法接受任意 `AsRef<[u8]>` 类型，兼容两者。
    ///
    /// # 参数
    /// * `data` - 实现 `AsRef<[u8]>` 的字节缓冲（如 `Bytes`、`BytesMut`、`&[u8]`）
    pub fn decode_from_bytes<T: AsRef<[u8]>>(data: T) -> Result<Self, String> {
        serde_json::from_slice(data.as_ref()).map_err(|e| format!("消息解码失败: {}", e))
    }

    /// 判断是否为 Ping 消息
    pub fn is_ping(&self) -> bool {
        self.msg_type == MessageType::Ping
    }

    /// 判断是否为 Pong 消息
    pub fn is_pong(&self) -> bool {
        self.msg_type == MessageType::Pong
    }

    /// 判断是否为心跳消息（Ping 或 Pong）
    pub fn is_heartbeat(&self) -> bool {
        self.is_ping() || self.is_pong()
    }

    /// 计算从消息时间戳到现在的延迟（毫秒）
    ///
    /// 用于 Ping/Pong 计算往返延迟
    pub fn elapsed_millis(&self) -> u64 {
        let now = unix_millis();
        now.saturating_sub(self.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_encode_decode() {
        let msg = TcpMessage::data("Hello, TCP!");
        let encoded = msg.encode().expect("编码失败");
        let decoded = TcpMessage::decode(&encoded).expect("解码失败");

        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.payload, decoded.payload);
        assert_eq!(msg.timestamp, decoded.timestamp);
    }

    #[test]
    fn test_ping_pong() {
        let ping = TcpMessage::ping();
        assert!(ping.is_ping());
        assert!(ping.is_heartbeat());

        let pong = TcpMessage::pong(ping.timestamp);
        assert!(pong.is_pong());
        assert!(pong.is_heartbeat());
    }

    #[test]
    fn test_custom_message() {
        let msg = TcpMessage::custom("Login", "user=alice");
        let encoded = msg.encode().expect("编码失败");
        let decoded = TcpMessage::decode(&encoded).expect("解码失败");

        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    fn test_decode_invalid_data() {
        let result = TcpMessage::decode(b"not a json");
        assert!(result.is_err());
    }
}
