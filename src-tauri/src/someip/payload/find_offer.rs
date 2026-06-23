//! FindOrOfferPayload 实现
//!
//! 对应 C++ `setFindOrOfferPayload`（someip_protocol.cpp:94-103）。
//!
//! # 字节布局
//!
//! ```text
//! Offset  Size  Field
//! 0       1     flag              标志位（0x80 | 0x40）
//! 1       3     reserved          保留字段（3 字节）
//! 4       4     entryLen          条目长度（u32 大端序）
//! 8       16    entry             SomeIPEntry（16 字节）
//! 24      4     optionsLen        选项长度（u32 大端序）
//! ```
//!
//! 共 28 字节（1 + 3 + 4 + 16 + 4）。
//!
//! # C++ 兼容性说明
//!
//! C++ 中 `reserved` 处理：
//! ```cpp
//! payload.append(reinterpret_cast<const char*>(&reserved), sizeof(reserved) - 1);
//! ```
//! `sizeof(reserved)` 是 `quint32` = 4 字节，减 1 = 3 字节。
//! 即只写入 `reserved` 的低 3 字节。

use crate::someip::entry::SomeIPEntry;
use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// FindOrOfferPayload（服务发现或提供服务）。
///
/// 对应 C++ `setFindOrOfferPayload`（someip_protocol.cpp:94-103）。
///
/// 用于 SD（Service Discovery）消息的 payload。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindOrOfferPayload {
    /// 标志位（默认 `0x80 | 0x40`）
    pub flag: u8,
    /// 保留字段（3 字节，默认 `0x00`）
    pub reserved: [u8; 3],
    /// 条目长度（大端序 u32，默认 `sizeof(SomeIPEntry)` = 16）
    pub entry_len: u32,
    /// 服务发现条目
    pub entry: SomeIPEntry,
    /// 选项长度（大端序 u32，默认 `0x00`）
    pub options_len: u32,
}

impl FindOrOfferPayload {
    /// 创建新的 FindOrOfferPayload。
    pub fn new(
        flag: u8,
        reserved: [u8; 3],
        entry_len: u32,
        entry: SomeIPEntry,
        options_len: u32,
    ) -> Self {
        FindOrOfferPayload {
            flag,
            reserved,
            entry_len: entry_len.to_be(), // C++ 中转为大端序
            entry,
            options_len: options_len.to_be(), // C++ 中直接写入（已是大端序？）
        }
    }

    /// 返回默认 FindOrOfferPayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        FindOrOfferPayload {
            flag: 0x80 | 0x40,
            reserved: [0x00, 0x00, 0x00],
            entry_len: (std::mem::size_of::<SomeIPEntry>() as u32).to_be(),
            entry: SomeIPEntry::default_entry(),
            options_len: 0_u32.to_be(),
        }
    }

    /// 序列化为字节数组。
    ///
    /// 对应 C++ `setFindOrOfferPayload()`（someip_protocol.cpp:94-103）。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(28);

        // flag: u8
        bytes.push(self.flag);

        // reserved: 3 字节（C++ 中写入 reserved 的低 3 字节）
        bytes.extend_from_slice(&self.reserved);

        // entryLen: u32 大端序
        bytes.extend_from_slice(&self.entry_len.to_be_bytes());

        // entry: SomeIPEntry（16 字节）
        bytes.extend_from_slice(&self.entry.to_bytes());

        // optionsLen: u32
        // NOTE: C++ 中直接 `append(reinterpret_cast<const char*>(&optionsLen), sizeof(optionsLen))`
        // optionsLen 是 quint32（本机序），直接写入
        // 为保持兼容，此处使用本机序
        bytes.extend_from_slice(&self.options_len.to_ne_bytes());

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 28 字节的字节数组
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 28` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 28 {
            return Err(SomeIPError::insufficient_buffer(28, bytes.len()));
        }

        let flag = bytes[0];
        let reserved = [bytes[1], bytes[2], bytes[3]];
        let entry_len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let entry = SomeIPEntry::from_bytes(&bytes[8..24])?;
        let options_len = u32::from_ne_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        Ok(FindOrOfferPayload {
            flag,
            reserved,
            entry_len,
            entry,
            options_len,
        })
    }
}

impl Payload for FindOrOfferPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::FindOrOffer
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for FindOrOfferPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for FindOrOfferPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_offer_payload_to_bytes_roundtrip() {
        let payload = FindOrOfferPayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 28);

        let parsed = FindOrOfferPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.flag, payload.flag);
    }
}
