//! CamExclusivePayload 实现
//!
//! 对应 C++ `setCamExclusivePayload`（someip_protocol.cpp:105-109）。
//!
//! # 字节布局（共 4 字节）
//!
//! ```text
//! Offset  Size  Field
//! 0       4     index             摄像头索引
//! ```
//!
//! # C++ 兼容性重要说明
//!
//! C++ 实现：
//! ```cpp
//! QByteArray SomeIPMessage::setCamExclusivePayload(quint32 index) {
//!     QByteArray payload;
//!     payload.append(reinterpret_cast<const char*>(&index), sizeof(index));
//!     return payload;
//! }
//! ```
//!
//! **注意**：C++ 中 `index` 直接使用本机序内存表示，未调用 `qToBigEndian`。
//! 这可能是 Bug，也可能是设备协议约定。
//!
//! Rust 移植时保持此行为（使用本机序），但添加 `// NOTE:` 注释标注。

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// CamExclusivePayload（摄像头独占模式）。
///
/// 对应 C++ `setCamExclusivePayload`（someip_protocol.cpp:105-109）。
///
/// # C++ 兼容性说明
///
/// C++ 中 `index` 未转大端序（可能是 Bug，也可能是协议约定）。
/// Rust 实现保持此行为，使用本机序。
///
/// **注意**：如果设备实际期望大端序，此处应改为 `to_be_bytes()`。
/// 建议与设备厂商确认。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CamExclusivePayload {
    /// 摄像头索引（默认 `0x0A`）
    ///
    /// C++ 默认值：`0x0A`。
    /// **注意**：此字段使用本机序（与 C++ 行为一致）。
    pub index: u32,
}

impl CamExclusivePayload {
    /// 创建新的 CamExclusivePayload。
    pub fn new(index: u32) -> Self {
        CamExclusivePayload { index }
    }

    /// 返回默认 CamExclusivePayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        CamExclusivePayload { index: 0x0A }
    }

    /// 序列化为字节数组。
    ///
    /// 对应 C++ `setCamExclusivePayload()`（someip_protocol.cpp:105-109）。
    ///
    /// # 注意
    ///
    /// C++ 中未转大端序，此处使用本机序以保持兼容。
    /// 如果设备实际期望大端序，应改用 `to_be_bytes()`。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);

        // NOTE: C++ 中未转大端序，此处使用本机序以保持兼容
        // 如果设备实际期望大端序，应改为：bytes.extend_from_slice(&self.index.to_be_bytes());
        bytes.extend_from_slice(&self.index.to_ne_bytes());

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 4 字节的字节数组
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 4` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 4 {
            return Err(SomeIPError::insufficient_buffer(4, bytes.len()));
        }

        // NOTE: 与 to_bytes() 一致，使用本机序
        let index = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        Ok(CamExclusivePayload { index })
    }
}

impl Payload for CamExclusivePayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::SetCamExclusive
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for CamExclusivePayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for CamExclusivePayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cam_exclusive_payload_to_bytes_roundtrip() {
        let payload = CamExclusivePayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 4);

        let parsed = CamExclusivePayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.index, payload.index);
    }
}
