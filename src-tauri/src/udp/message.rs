//! UDP 消息协议模块
//!
//! 该模块定义了 UDP 通信中使用的消息格式和编解码逻辑，
//! 支持 JSON 序列化的结构化消息，包含消息类型、负载和时间戳。
//!
//! # 消息格式
//!
//! 消息使用 JSON 编码为 UTF-8 字节流，格式如下：
//! ```json
//! {
//!     "msg_type": "Data",
//!     "payload": "Hello, UDP!",
//!     "timestamp": 1719000000000
//! }
//! ```
//!
//! 同时也支持原始字节（Raw）模式，直接传输二进制数据。

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 获取当前 Unix 时间戳（毫秒）
fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// UDP 消息类型
///
/// 定义不同类型的 UDP 消息，用于区分消息用途
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// 普通数据消息
    Data,
    /// 广播消息
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

/// UDP 结构化消息
///
/// 使用 JSON 编码的消息格式，便于解析和扩展
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpMessage {
    /// 消息类型
    pub msg_type: MessageType,
    /// 消息负载（文本内容）
    pub payload: String,
    /// 时间戳（Unix 毫秒）
    pub timestamp: u64,
}

impl UdpMessage {
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
    /// # 返回值
    /// 返回 Result，成功时包含编码后的字节向量
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("消息编码失败: {}", e))
    }

    /// 从字节序列解码消息（JSON 格式）
    ///
    /// # 参数
    /// * `data` - 原始字节序列
    ///
    /// # 返回值
    /// 返回 Result，成功时包含解码后的消息
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| format!("消息解码失败: {}", e))
    }

    /// 判断是否为 Ping 消息
    pub fn is_ping(&self) -> bool {
        self.msg_type == MessageType::Ping
    }

    /// 判断是否为 Pong 消息
    pub fn is_pong(&self) -> bool {
        self.msg_type == MessageType::Pong
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
        let msg = UdpMessage::data("Hello, UDP!");
        let encoded = msg.encode().expect("编码失败");
        let decoded = UdpMessage::decode(&encoded).expect("解码失败");

        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.payload, decoded.payload);
        assert_eq!(msg.timestamp, decoded.timestamp);
    }

    #[test]
    fn test_ping_pong() {
        let ping = UdpMessage::ping();
        assert!(ping.is_ping());

        let pong = UdpMessage::pong(ping.timestamp);
        assert!(pong.is_pong());
    }

    #[test]
    fn test_custom_message() {
        let msg = UdpMessage::custom("Login", "user=alice");
        let encoded = msg.encode().expect("编码失败");
        let decoded = UdpMessage::decode(&encoded).expect("解码失败");

        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    fn test_decode_invalid_data() {
        let result = UdpMessage::decode(b"not a json");
        assert!(result.is_err());
    }
}
