//! SomeIP 消息构建器
//!
//! 替代 C++ 上帝类 `SomeIPMessage` + 大 `switch` 分发。
//!
//! # 设计说明
//!
//! C++ 中 `SomeIPMessage::Build()` 用大 `switch` 按方法 ID 设置 `length` 并拼接 payload。
//! Rust 中用 trait 多态替代：
//!
//! - `build(&dyn Payload)`：调用 `payload.method_id()` 设置 header，调用 `payload.encode()` 获取 payload 字节
//! - 一行代码替代整个 `switch`
//!
//! # 使用示例
//!
//! ```rust
//! # use crate::someip::message::SomeIPMessage;
//! # use crate::someip::payload::MediaPayload;
//! let mut msg = SomeIPMessage::new(0x433F, 0x0001, 0x01, 0x01, 0x01, 0x00);
//! let payload = MediaPayload::default_payload();
//! let bytes = msg.build(&payload);
//! ```

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::header::SomeIPHeader;
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::Payload;
use crate::someip::payload::EmptyPayload;

/// SomeIP 消息构建器。
///
/// 替代 C++ `SomeIPMessage` 上帝类。
///
/// # 设计要点
///
/// - 用 `build(&dyn Payload)` 替代 C++ 的 `Build()` 大 `switch`
/// - 用 trait 多态替代硬编码的方法 ID 分支
/// - Get 类请求（无 payload）使用 `EmptyPayload`
#[derive(Debug, Clone)]
pub struct SomeIPMessage {
    /// SomeIP 报文头
    pub header: SomeIPHeader,
}

impl SomeIPMessage {
    /// 创建新的 SomeIP 消息构建器。
    ///
    /// 对应 C++ `SomeIPMessage::SetHeader()`（someip_protocol.cpp:12-14）。
    ///
    /// # 参数
    ///
    /// * `service_id` - 服务 ID（本机序）
    /// * `client_id` - 客户端 ID（本机序）
    /// * `session_id` - 会话 ID（本机序）
    /// * `message_type` - 消息类型
    /// * `return_code` - 返回码
    pub fn new(
        service_id: u16,
        client_id: u16,
        session_id: u16,
        message_type: u8,
        return_code: u8,
    ) -> Self {
        SomeIPMessage {
            header: SomeIPHeader::new(
                service_id,
                0x0000, // method_id 由 build() 设置
                client_id,
                session_id,
                message_type,
                return_code,
            ),
        }
    }

    /// 构建完整消息（替代 C++ `Build()` 的 `switch`）。
    ///
    /// 对应 C++ `SomeIPMessage::Build()`（someip_protocol.cpp:16-85）。
    ///
    /// # 设计说明
    ///
    /// C++ 中用大 `switch` 按方法 ID 设置 `length`：
    /// - Get 类方法：`SetLength(0)`（无 payload）
    /// - Set 类方法：`SetLength(payload.size())`（有 payload）
    ///
    /// Rust 中用 trait 多态替代：
    /// 1. 调用 `payload.method_id()` 获取方法 ID，设置到 header
    /// 2. 调用 `payload.encode()` 获取 payload 字节
    /// 3. 根据 payload 字节长度设置 header.length
    /// 4. 拼接 header + payload
    ///
    /// **一行代码替代整个 `switch`**。
    ///
    /// # 参数
    ///
    /// * `payload` - 实现 `Payload` trait 的对象（动态分发）
    ///
    /// # 返回
    ///
    /// 完整的 SomeIP 消息字节序列（header + payload）。
    pub fn build(&mut self, payload: &dyn Payload) -> Vec<u8> {
        let payload_bytes = payload.encode();
        self.header.set_method_id(payload.method_id());
        self.header.set_length(payload_bytes.len() as u32);

        let mut msg = self.header.to_bytes();
        msg.extend_from_slice(&payload_bytes);
        msg
    }

    /// 构建 Get 类请求（无 payload，length=0）。
    ///
    /// 对应 C++ `Build()` 中 `SetLength(0)` 的分支。
    ///
    /// # 参数
    ///
    /// * `method` - 方法 ID
    pub fn build_get(&mut self, method: SomeIPMethod) -> Vec<u8> {
        let payload = EmptyPayload::new(method);
        self.build(&payload)
    }

    /// 从字节数组解析 SomeIP 消息。
    ///
    /// 对应 C++ `SomeIPMessage::ToByteArray()` 的反向操作。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 16 字节的字节数组
    ///
    /// # 返回
    ///
    /// `(SomeIPHeader, &[u8])`：解析后的 header 和 payload 切片。
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 16` 时返回 `InsufficientBuffer`。
    pub fn parse(bytes: &[u8]) -> SomeIPResult<(SomeIPHeader, Vec<u8>)> {
        if bytes.len() < 16 {
            return Err(SomeIPError::insufficient_buffer(16, bytes.len()));
        }

        let header = SomeIPHeader::from_bytes(bytes)?;
        let payload_len = header.get_length() - 8; // length 包含 header 中除 length 外的 8 字节

        if bytes.len() < 16 + payload_len as usize {
            return Err(SomeIPError::insufficient_buffer(
                16 + payload_len as usize,
                bytes.len(),
            ));
        }

        let payload = bytes[16..16 + payload_len as usize].to_vec();
        Ok((header, payload))
    }

    /// 序列化消息为字节数组。
    ///
    /// 对应 C++ `SomeIPMessage::ToByteArray()`（someip_protocol.cpp:87-92）。
    ///
    /// 注意：此方法需要事先设置 `self.header` 和 `self.data`。
    /// 推荐使用 `build()` 方法替代。
    pub fn to_bytes(&self) -> Vec<u8> {
        // NOTE: C++ 中还有 `data` 字段，但 Rust 实现中 payload 由 build() 拼接
        // 此处保留接口兼容性
        self.header.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::someip::payload::MediaPayload;

    #[test]
    fn test_build_media_payload() {
        let mut msg = SomeIPMessage::new(0x433F, 0x0001, 0x0001, 0x01, 0x00);
        let payload = MediaPayload::default_payload();
        let bytes = msg.build(&payload);

        // 验证总长度：16 (header) + 32 (payload) = 48
        assert_eq!(bytes.len(), 48);

        // 验证 method_id
        let method_id = u16::from_be_bytes([bytes[2], bytes[3]]);
        assert_eq!(method_id, SomeIPMethod::SetMedia as u16);
    }

    #[test]
    fn test_build_get_request() {
        let mut msg = SomeIPMessage::new(0x433F, 0x0001, 0x0001, 0x01, 0x00);
        let bytes = msg.build_get(SomeIPMethod::GetMedia);

        // Get 请求无 payload，总长度 = 16
        assert_eq!(bytes.len(), 16);

        // 验证 length = 0（offset 4-7）
        let length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(length, 8); // length + 8 = 8 → length = 0
    }
}
