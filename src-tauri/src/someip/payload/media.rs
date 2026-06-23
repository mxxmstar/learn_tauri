//! MediaPayload 实现
//!
//! 对应 C++ `MediaPayload`（someip_client.h:124-157）。
//!
//! # 字节布局（大端序，共 32 字节）
//!
//! ```text
//! Offset  Size  Field
//! 0       4     enableMirror       是否镜像（bool → u32）
//! 4       4     enableFlip         是否翻转（bool → u32）
//! 8       4     encode             编码格式（u8 → u32）
//! 12      4     resolution         分辨率（u8 → u32）
//! 16      4     fps                帧率（u8 → u32）
//! 20      4     bitrate            码率（u16 → u32）
//! 24      4     rcMode             RC 模式（u8 → u32）
//! 28      4     IFrameInterval     I 帧间隔（u8 → u32）
//! ```

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// MediaPayload（媒体配置）。
///
/// 对应 C++ `MediaPayload`（someip_client.h:124-157）。
///
/// 每个字段在序列化时转为 4 字节大端序 `u32`，共 32 字节。
///
/// # C++ 兼容性说明
///
/// C++ 中每个字段都用 `qToBigEndian<quint32>()` 转换，即：
/// - `bool` → `u32`（0 或 1）→ 大端序
/// - `quint8` → `u32` → 大端序
/// - `quint16` → `u32` → 大端序
///
/// Rust 实现严格遵循此规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaPayload {
    /// 是否镜像（默认 `true`）
    pub enable_mirror: bool,
    /// 是否翻转（默认 `false`）
    pub enable_flip: bool,
    /// 编码格式（默认 `0x00`）
    pub encode: u8,
    /// 分辨率（默认 `0x00`）
    pub resolution: u8,
    /// 帧率（默认 `30`）
    pub fps: u8,
    /// 码率（默认 `0x1000`）
    pub bitrate: u16,
    /// RC 模式（默认 `0x00`）
    pub rc_mode: u8,
    /// I 帧间隔（默认 `30`）
    pub i_frame_interval: u8,
}

impl MediaPayload {
    /// 创建新的 MediaPayload。
    pub fn new(
        enable_mirror: bool,
        enable_flip: bool,
        encode: u8,
        resolution: u8,
        fps: u8,
        bitrate: u16,
        rc_mode: u8,
        i_frame_interval: u8,
    ) -> Self {
        MediaPayload {
            enable_mirror,
            enable_flip,
            encode,
            resolution,
            fps,
            bitrate,
            rc_mode,
            i_frame_interval,
        }
    }

    /// 返回默认 MediaPayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        MediaPayload {
            enable_mirror: true,
            enable_flip: false,
            encode: 0x00,
            resolution: 0x00,
            fps: 30,
            bitrate: 0x1000,
            rc_mode: 0x00,
            i_frame_interval: 30,
        }
    }

    /// 序列化为字节数组（大端序）。
    ///
    /// 对应 C++ `MediaPayload::ToByteArray()`（someip_client.h:136-156）。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);

        // enableMirror: bool → u32 → 大端序
        let val: u32 = if self.enable_mirror { 1 } else { 0 };
        bytes.extend_from_slice(&val.to_be_bytes());

        // enableFlip: bool → u32 → 大端序
        let val: u32 = if self.enable_flip { 1 } else { 0 };
        bytes.extend_from_slice(&val.to_be_bytes());

        // encode: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.encode as u32).to_be_bytes());

        // resolution: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.resolution as u32).to_be_bytes());

        // fps: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.fps as u32).to_be_bytes());

        // bitrate: u16 → u32 → 大端序
        bytes.extend_from_slice(&(self.bitrate as u32).to_be_bytes());

        // rcMode: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.rc_mode as u32).to_be_bytes());

        // IFrameInterval: u8 → u32 → 大端序
        bytes.extend_from_slice(&(self.i_frame_interval as u32).to_be_bytes());

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// 对应 C++ `parseMediaPayload()`（someip_protocol.cpp:166-183）。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 32 字节的字节数组（大端序）
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 32` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 32 {
            return Err(SomeIPError::insufficient_buffer(32, bytes.len()));
        }

        let enable_mirror = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != 0;
        let enable_flip = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) != 0;
        let encode = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as u8;
        let resolution = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as u8;
        let fps = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as u8;
        let bitrate = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as u16;
        let rc_mode = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as u8;
        let i_frame_interval = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as u8;

        Ok(MediaPayload {
            enable_mirror,
            enable_flip,
            encode,
            resolution,
            fps,
            bitrate,
            rc_mode,
            i_frame_interval,
        })
    }
}

impl Payload for MediaPayload {
    fn method_id(&self) -> SomeIPMethod {
        // MediaPayload 可用于 SetMedia 或 GetMedia（响应）
        // 默认返回 SetMedia，实际使用时由调用者指定
        SomeIPMethod::SetMedia
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for MediaPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for MediaPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_payload_to_bytes_roundtrip() {
        let payload = MediaPayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 32);

        let parsed = MediaPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.enable_mirror, payload.enable_mirror);
        assert_eq!(parsed.bitrate, payload.bitrate);
    }
}
