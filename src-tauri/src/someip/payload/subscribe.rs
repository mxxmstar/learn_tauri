//! SubscribePayload 实现
//!
//! 对应 C++ `SubscribePayload`（someip_client.h:325-335）。
//!
//! # 字节布局（大端序，共 4 字节）
//!
//! ```text
//! Offset  Size  Field
//! 0       4     index             订阅索引（u8 → u32 大端序）
//! ```

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// SubscribePayload（订阅事件）。
///
/// 对应 C++ `SubscribePayload`（someip_client.h:325-335）。
///
/// `index`（u8）在序列化时转为 4 字节大端序 `u32`。
///
/// # C++ 兼容性说明
///
/// C++ 实现：
/// ```cpp
/// quint32 val = qToBigEndian<quint32>(index);
/// byteArray.append(reinterpret_cast<const char*>(&val), sizeof(val));
/// ```
/// 即 `u8` → `u32` → 大端序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscribePayload {
    /// 订阅索引（默认 `0x01`）
    pub index: u8,
}

impl SubscribePayload {
    /// 创建新的 SubscribePayload。
    pub fn new(index: u8) -> Self {
        SubscribePayload { index }
    }

    /// 返回默认 SubscribePayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        SubscribePayload { index: 0x01 }
    }

    /// 序列化为字节数组（大端序）。
    ///
    /// 对应 C++ `SubscribePayload::ToByteArray()`（someip_client.h:329-334）。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);
        // index: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.index as u32).to_be_bytes());
        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 4 字节的字节数组（大端序）
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 4` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 4 {
            return Err(SomeIPError::insufficient_buffer(4, bytes.len()));
        }

        let index = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u8;
        Ok(SubscribePayload { index })
    }
}

impl Payload for SubscribePayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::Subscribe
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for SubscribePayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for SubscribePayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_payload_to_bytes_roundtrip() {
        let payload = SubscribePayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 4);

        let parsed = SubscribePayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.index, payload.index);
    }
}
