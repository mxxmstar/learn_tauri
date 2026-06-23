//! SomeIP 报文头定义（16 字节固定头，大端序）
//!
//! 对应 C++ `SomeIPHeader`（someip_protocol.h:7-31）。
//!
//! # 字节布局（大端序）
//!
//! ```text
//! Offset  Size  Field
//! 0       2     serviceId          服务 ID
//! 2       2     methodId           方法 ID
//! 4       4     length             从 length 字段到消息结束的字节数（含 length 本身？不含，C++ SetLength 加 8）
//! 8       2     clientId           客户端 ID
//! 10      2     sessionId          会话 ID
//! 12      1     protocolVersion    协议版本（固定 0x01）
//! 13      1     interfaceVersion   接口版本
//! 14      1     messageType        消息类型
//! 15      1     returnCode         返回码
//! ```
//!
//! 共 16 字节。

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;

/// SomeIP 报文头（16 字节固定头）。
///
/// 对应 C++ `SomeIPHeader`（someip_protocol.h:7-31）。
///
/// # C++ 兼容性说明
///
/// C++ 版本在构造函数中将所有多字节字段转为大端序存储（`qToBigEndian`）。
/// Rust 版本采用相同策略：构造时转大端序，`to_bytes()` 直接拷贝内存。
/// 这保证了与 C++ 设备的二进制兼容。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SomeIPHeader {
    /// 服务 ID（16 bit，大端序存储）
    pub service_id: u16,
    /// 方法 ID（16 bit，大端序存储）
    pub method_id: u16,
    /// 数据长度：从 length 字段开始到消息结束的字节数（32 bit，大端序存储）
    ///
    /// C++ `SetLength` 实现：`this->length = qToBigEndian<quint32>(length + 8);`
    /// 即 length 字段的值等于 payload 长度 + 8（header 中除 length 外的 8 字节）。
    /// 注意：C++ 注释说"从 length 字段开始"，但实际加了 8，这是 SomeIP 标准定义。
    pub length: u32,
    /// 客户端 ID（16 bit，大端序存储）
    pub client_id: u16,
    /// 会话 ID（16 bit，大端序存储）
    pub session_id: u16,
    /// 协议版本（8 bit，固定为 0x01）
    pub protocol_version: u8,
    /// 接口版本（8 bit，用于标识服务升级）
    pub interface_version: u8,
    /// 消息类型（8 bit）
    pub message_type: u8,
    /// 返回码（8 bit）
    pub return_code: u8,
}

impl SomeIPHeader {
    /// 服务 ID 默认值（对应 C++ 中的 0x433F？C++ SomeIPEntry 中 serviceId=0x433F，但 header 中未指定）
    pub const DEFAULT_SERVICE_ID: u16 = 0x433F;

    /// 协议版本默认值
    pub const PROTOCOL_VERSION: u8 = 0x01;

    /// 创建新的 SomeIP 报文头。
    ///
    /// 所有多字节字段在构造时以本机序存储，序列化时转为大端序。
    /// 这是 Rust 的惯用做法，避免混淆。
    ///
    /// # 参数
    ///
    /// * `service_id` - 服务 ID（本机序）
    /// * `method_id` - 方法 ID（本机序）
    /// * `client_id` - 客户端 ID（本机序）
    /// * `session_id` - 会话 ID（本机序）
    /// * `message_type` - 消息类型
    /// * `return_code` - 返回码
    pub fn new(
        service_id: u16,
        method_id: u16,
        client_id: u16,
        session_id: u16,
        message_type: u8,
        return_code: u8,
    ) -> Self {
        SomeIPHeader {
            service_id,  // 本机序存储
            method_id,   // 本机序存储
            length: 0,   // 初始为 0，后续调用 set_length 设置
            client_id,   // 本机序存储
            session_id,  // 本机序存储
            protocol_version: Self::PROTOCOL_VERSION,
            interface_version: 0x01,
            message_type,
            return_code,
        }
    }

    /// 设置 length 字段。
    ///
    /// C++ 实现：`this->length = qToBigEndian<quint32>(length + 8);`
    ///
    /// SomeIP 标准中，length 字段表示"从 request ID（clientId+sessionId）开始到消息结束的字节数"，
    /// 即 payload 长度 + 8（clientId(2) + sessionId(2) + protocolVersion(1) + interfaceVersion(1) + messageType(1) + returnCode(1)）。
    ///
    /// # 参数
    ///
    /// * `payload_len` - payload 的字节长度（本机序）
    pub fn set_length(&mut self, payload_len: u32) {
        self.length = payload_len + 8; // 存储为本机序
    }

    /// 获取 length 字段的值（本机序）。
    pub fn get_length(&self) -> u32 {
        self.length
    }

    /// 序列化为 16 字节大端序字节数组。
    ///
    /// 对应 C++ `reinterpret_cast<const char*>(&header)`。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        // 所有多字节字段转大端序写入
        bytes.extend_from_slice(&self.service_id.to_be_bytes());
        bytes.extend_from_slice(&self.method_id.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.client_id.to_be_bytes());
        bytes.extend_from_slice(&self.session_id.to_be_bytes());
        bytes.push(self.protocol_version);
        bytes.push(self.interface_version);
        bytes.push(self.message_type);
        bytes.push(self.return_code);
        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// 字节数组应为大端序，函数内转为本机序存储。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 16 字节的字节数组（大端序）
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 16` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 16 {
            return Err(SomeIPError::insufficient_buffer(16, bytes.len()));
        }

        // 从大端序字节数组转为本机序
        let service_id = u16::from_be_bytes([bytes[0], bytes[1]]);
        let method_id = u16::from_be_bytes([bytes[2], bytes[3]]);
        let length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let client_id = u16::from_be_bytes([bytes[8], bytes[9]]);
        let session_id = u16::from_be_bytes([bytes[10], bytes[11]]);

        Ok(SomeIPHeader {
            service_id,  // 本机序存储
            method_id,   // 本机序存储
            length,      // 本机序存储
            client_id,   // 本机序存储
            session_id,  // 本机序存储
            protocol_version: bytes[12],
            interface_version: bytes[13],
            message_type: bytes[14],
            return_code: bytes[15],
        })
    }

    /// 获取方法 ID（本机序）。
    pub fn get_method_id(&self) -> SomeIPMethod {
        SomeIPMethod::from(self.method_id)
    }

    /// 设置方法 ID（本机序输入）。
    pub fn set_method_id(&mut self, method: SomeIPMethod) {
        self.method_id = method as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_to_bytes_roundtrip() {
        let mut header = SomeIPHeader::new(0x433F, 0x0172, 0x0001, 0x0001, 0x01, 0x00);
        header.set_length(32); // MediaPayload 32 字节

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 16);

        let parsed = SomeIPHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.get_method_id(), SomeIPMethod::SetMedia);
        assert_eq!(parsed.get_length(), 40); // 32 + 8
    }
}
