//! AlgROIPayload 实现
//!
//! 对应 C++ `AlgROIPayload`（someip_client.h:159-179）。
//!
//! # 字节布局（共 196 字节）
//!
//! ```text
//! Offset  Size  Field
//! 0       96     roiList           12 个点 (x, y: f64) = 12 * 16 = 192 字节
//! 192     4     roiType           ROI 类型（u32 大端序）
//! ```
//!
//! 共 196 字节（192 + 4）。
//!
//! # C++ 字节序重要说明
//!
//! C++ 注释明确标注："注意，这里的点坐标未转成大端序！！！！！"
//! 即 `double` 值直接使用本机序（小端序系统上为小端序）。
//!
//! Rust 实现必须保持此行为，使用 `f64::to_ne_bytes()` 而非 `to_be_bytes()`，
//! 否则无法与现有设备通信。

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// AlgROIPayload（算法 ROI 区域）。
///
/// 对应 C++ `AlgROIPayload`（someip_client.h:159-179）。
///
/// # C++ 兼容性说明
///
/// C++ 中 `double` 值未转大端序（直接使用本机序内存表示）。
/// Rust 实现必须使用 `f64::to_ne_bytes()` 保持兼容。
///
/// ROI 坐标点为 12 个 `(x, y)` 对，对应 4 个点的 3 种颜色？或其他含义。
/// 根据 C++ 代码，12 个点可能是 4 个四边形（每个四边形 3 个点？）或 12 个控制点。
#[derive(Debug, Clone, PartialEq)]
pub struct AlgROIPayload {
    /// ROI 坐标点列表（12 个点，每个点 `(x, y)`）
    ///
    /// 对应 C++ `QVector<QPair<double, double>> roiList`。
    pub roi_list: [(f64, f64); 12],
    /// ROI 类型（默认 `0x00`：梯形，0x01：矩形，0x02：矩形？）
    ///
    /// 对应 C++ `quint8 roiType`。
    /// 注意：C++ 中 `roiType` 是 `quint8`，但序列化时转为 `quint32` 大端序。
    pub roi_type: u8,
}

impl AlgROIPayload {
    /// 创建新的 AlgROIPayload。
    pub fn new(roi_list: [(f64, f64); 12], roi_type: u8) -> Self {
        AlgROIPayload {
            roi_list,
            roi_type,
        }
    }

    /// 返回默认 AlgROIPayload。
    pub fn default_payload() -> Self {
        AlgROIPayload {
            roi_list: [(0.0, 0.0); 12],
            roi_type: 0x00,
        }
    }

    /// 序列化为字节数组。
    ///
    /// 对应 C++ `AlgROIPayload::ToByteArray()`（someip_client.h:164-178）。
    ///
    /// # 字节序规则
    ///
    /// - `double` 值：使用本机序（`to_ne_bytes`），与 C++ 行为一致
    /// - `roiType`：转为 `u32` 大端序
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(196);

        // roiList: 12 个点 (x, y: f64)，使用本机序（与 C++ 兼容）
        for (x, y) in &self.roi_list {
            bytes.extend_from_slice(&x.to_ne_bytes());
            bytes.extend_from_slice(&y.to_ne_bytes());
        }

        // roiType: u8 → u32 → 大端序
        let type_bytes = (self.roi_type as u32).to_be_bytes();
        bytes.extend_from_slice(&type_bytes);

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// 对应 C++ `parseAlgROIPayload()`（someip_protocol.cpp:194-210）。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 196 字节的字节数组
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 196` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 196 {
            return Err(SomeIPError::insufficient_buffer(196, bytes.len()));
        }

        let mut roi_list = [(0.0, 0.0); 12];

        // roiList: 12 个点 (x, y: f64)，使用本机序
        for (i, item) in roi_list.iter_mut().enumerate() {
            let offset = i * 16;
            let x = f64::from_ne_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            let y = f64::from_ne_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]);
            *item = (x, y);
        }

        // roiType: u32 大端序 → u8
        let roi_type = u32::from_be_bytes([bytes[192], bytes[193], bytes[194], bytes[195]]) as u8;

        Ok(AlgROIPayload {
            roi_list,
            roi_type,
        })
    }
}

impl Payload for AlgROIPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::SetAlgROI
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for AlgROIPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for AlgROIPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alg_roi_payload_to_bytes_roundtrip() {
        let mut roi_list = [(0.0, 0.0); 12];
        roi_list[0] = (100.0, 200.0);
        roi_list[1] = (300.0, 400.0);

        let payload = AlgROIPayload::new(roi_list, 0x01);
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 196);

        let parsed = AlgROIPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.roi_list[0].0, 100.0);
        assert_eq!(parsed.roi_type, 0x01);
    }
}
