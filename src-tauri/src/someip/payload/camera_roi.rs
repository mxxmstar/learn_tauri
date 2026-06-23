//! CameraROI Payload 实现
//!
//! 对应 C++ `SetCameraROIPayload` 和 `GetCameraROIPayload`（someip_client.h:181-268）。
//!
//! # 字节布局（逐字段大端序，共 53 字节）
//!
//! 注意：C++ 中使用 `#pragma pack(push, 1)` 平铺结构体，
//! 部分字段已在 C++ 中转为大端序（如 `index = qToBigEndian<quint32>(0x01)`），
//! 部分字段未转（如 `quad1 = 0`）。
//!
//! Rust 实现采用逐字段序列化，严格遵循 C++ 的字节序规则。
//!
//! ```text
//! Offset  Size  Field
//! 0       4     index             索引（大端序 u32）
//! 4       2     quad1             ROI 区域坐标1（本机序 u16）
//! 6       2     quad2             ROI 区域坐标2（本机序 u16）
//! 8       2     quad3             ROI 区域坐标3（本机序 u16）
//! 10      2     quad4             ROI 区域坐标4（本机序 u16）
//! 12      1     enable            使能
//! 13      1     transMethod       传输方法
//! 14      2     transCycle        传输周期（大端序 u16）
//! 16      2     width             宽度（大端序 u16）
//! 18      2     height            高度（大端序 u16）
//! 20      4     frameRate         帧率（大端序 u32）
//! 24      1     interlaced        隔行扫描
//! 25      1     colorSpace        色彩空间
//! 26      4     maxBitrate        最大码率（大端序 u32）
//! 30      1     videoCompression  视频编码
//! 31      1     histogramEnable   直方图使能
//! 32      1     histogramUpdateCycle 直方图更新周期
//! 33      1     usedVideoComponent 使用的视频组件
//! 34      1     dataType          数据类型
//! 35      1     binSize           箱大小
//! 36      2     numberOfBins      箱数量（大端序 u16）
//! 38      1     blockEnable       块使能
//! 39      1     brightness        亮度
//! 40      1     contrast          对比度
//! 41      1     saturation        饱和度
//! 42      1     sharpness         锐度
//! 43      1     streamAtBoot      启动时流式传输
//! 44      1     algStatusIndicator 算法状态指示器
//! ```
//!
//! 共 45 字节（不含填充，因为 `#pragma pack(1)`）。

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};

/// SetCameraROIPayload（设置摄像头 ROI 区域）。
///
/// 对应 C++ `SetCameraROIPayload`（someip_client.h:181-225）。
///
/// # C++ 字节序说明
///
/// C++ 中部分字段已在构造时转为大端序：
/// - `index = qToBigEndian<quint32>(0x01)`
/// - `transCycle = qToBigEndian<quint16>(0)`
/// - `width = qToBigEndian<quint16>(1280)`
/// - `height = qToBigEndian<quint16>(800)`
/// - `frameRate = qToBigEndian<quint32>(30 << 16)`
/// - `maxBitrate = qToBigEndian<quint32>(10)`
/// - `numberOfBins = qToBigEndian<quint16>(0x00)`
///
/// 部分字段未转（本机序）：
/// - `quad1`, `quad2`, `quad3`, `quad4`
/// - `enable`, `transMethod`, `interlaced`, `colorSpace`, `videoCompression`
/// - `histogramEnable`, `histogramUpdateCycle`, `usedVideoComponent`, `dataType`, `binSize`
/// - `blockEnable`, `brightness`, `contrast`, `saturation`, `sharpness`, `streamAtBoot`, `algStatusIndicator`
///
/// Rust 实现严格遵循此规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetCameraROIPayload {
    /// 索引（大端序 u32，默认 `0x01`）
    pub index: u32,
    /// ROI 区域坐标 1（本机序 u16，默认 `0`）
    pub quad1: u16,
    /// ROI 区域坐标 2（本机序 u16，默认 `0`）
    pub quad2: u16,
    /// ROI 区域坐标 3（本机序 u16，默认 `0`）
    pub quad3: u16,
    /// ROI 区域坐标 4（本机序 u16，默认 `0`）
    pub quad4: u16,
    /// 使能（默认 `1`）
    pub enable: u8,
    /// 传输方法（默认 `0`）
    pub trans_method: u8,
    /// 传输周期（大端序 u16，默认 `0`）
    pub trans_cycle: u16,
    /// 宽度（大端序 u16，默认 `1280`）
    pub width: u16,
    /// 高度（大端序 u16，默认 `800`）
    pub height: u16,
    /// 帧率（大端序 u32，默认 `30 << 16`）
    pub frame_rate: u32,
    /// 隔行扫描（默认 `0`，0=逐行，1=隔行）
    pub interlaced: u8,
    /// 色彩空间（默认 `0x80`）
    pub color_space: u8,
    /// 最大码率（大端序 u32，默认 `10`）
    pub max_bitrate: u32,
    /// 视频编码（默认 `0x02`）
    pub video_compression: u8,
    /// 直方图使能（默认 `0x00`）
    pub histogram_enable: u8,
    /// 直方图更新周期（默认 `0x00`）
    pub histogram_update_cycle: u8,
    /// 使用的视频组件（默认 `0x00`）
    pub used_video_component: u8,
    /// 数据类型（默认 `0x04`）
    pub data_type: u8,
    /// 箱大小（默认 `0x01`）
    pub bin_size: u8,
    /// 箱数量（大端序 u16，默认 `0x00`）
    pub number_of_bins: u16,
    /// 块使能（默认 `true`）
    pub block_enable: bool,
    /// 亮度（默认 `50`）
    pub brightness: u8,
    /// 对比度（默认 `50`）
    pub contrast: u8,
    /// 饱和度（默认 `50`）
    pub saturation: u8,
    /// 锐度（默认 `50`）
    pub sharpness: u8,
    /// 启动时流式传输（默认 `false`）
    pub stream_at_boot: bool,
    /// 算法状态指示器（默认 `true`）
    pub alg_status_indicator: bool,
}

impl SetCameraROIPayload {
    /// 创建新的 SetCameraROIPayload。
    pub fn new(
        index: u32,
        quad1: u16,
        quad2: u16,
        quad3: u16,
        quad4: u16,
        enable: u8,
        trans_method: u8,
        trans_cycle: u16,
        width: u16,
        height: u16,
        frame_rate: u32,
        interlaced: u8,
        color_space: u8,
        max_bitrate: u32,
        video_compression: u8,
        histogram_enable: u8,
        histogram_update_cycle: u8,
        used_video_component: u8,
        data_type: u8,
        bin_size: u8,
        number_of_bins: u16,
        block_enable: bool,
        brightness: u8,
        contrast: u8,
        saturation: u8,
        sharpness: u8,
        stream_at_boot: bool,
        alg_status_indicator: bool,
    ) -> Self {
        SetCameraROIPayload {
            index: index.to_be(), // C++ 中已转大端序
            quad1,
            quad2,
            quad3,
            quad4,
            enable,
            trans_method,
            trans_cycle: trans_cycle.to_be(), // C++ 中已转大端序
            width: width.to_be(), // C++ 中已转大端序
            height: height.to_be(), // C++ 中已转大端序
            frame_rate: frame_rate.to_be(), // C++ 中已转大端序
            interlaced,
            color_space,
            max_bitrate: max_bitrate.to_be(), // C++ 中已转大端序
            video_compression,
            histogram_enable,
            histogram_update_cycle,
            used_video_component,
            data_type,
            bin_size,
            number_of_bins: number_of_bins.to_be(), // C++ 中已转大端序
            block_enable,
            brightness,
            contrast,
            saturation,
            sharpness,
            stream_at_boot,
            alg_status_indicator,
        }
    }

    /// 返回默认 SetCameraROIPayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        SetCameraROIPayload {
            index: 0x01_u32.to_be(),
            quad1: 0,
            quad2: 0,
            quad3: 0,
            quad4: 0,
            enable: 1,
            trans_method: 0,
            trans_cycle: 0_u16.to_be(),
            width: 1280_u16.to_be(),
            height: 800_u16.to_be(),
            frame_rate: (30u32 << 16).to_be(), // 30 << 16 = 0x001E0000
            interlaced: 0,
            color_space: 0x80,
            max_bitrate: 10_u32.to_be(),
            video_compression: 0x02,
            histogram_enable: 0x00,
            histogram_update_cycle: 0x00,
            used_video_component: 0x00,
            data_type: 0x04,
            bin_size: 0x01,
            number_of_bins: 0_u16.to_be(),
            block_enable: true,
            brightness: 50,
            contrast: 50,
            saturation: 50,
            sharpness: 50,
            stream_at_boot: false,
            alg_status_indicator: true,
        }
    }

    /// 序列化为字节数组。
    ///
    /// 严格遵循 C++ 的字节序规则。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(45);

        // index: 大端序 u32（C++ 中已转）
        bytes.extend_from_slice(&self.index.to_be_bytes());

        // quad1-4: 本机序 u16（C++ 中未转）
        bytes.extend_from_slice(&self.quad1.to_ne_bytes());
        bytes.extend_from_slice(&self.quad2.to_ne_bytes());
        bytes.extend_from_slice(&self.quad3.to_ne_bytes());
        bytes.extend_from_slice(&self.quad4.to_ne_bytes());

        // enable, transMethod: u8
        bytes.push(self.enable);
        bytes.push(self.trans_method);

        // transCycle: 大端序 u16（C++ 中已转）
        bytes.extend_from_slice(&self.trans_cycle.to_be_bytes());

        // width, height: 大端序 u16（C++ 中已转）
        bytes.extend_from_slice(&self.width.to_be_bytes());
        bytes.extend_from_slice(&self.height.to_be_bytes());

        // frameRate: 大端序 u32（C++ 中已转）
        bytes.extend_from_slice(&self.frame_rate.to_be_bytes());

        // interlaced, colorSpace: u8
        bytes.push(self.interlaced);
        bytes.push(self.color_space);

        // maxBitrate: 大端序 u32（C++ 中已转）
        bytes.extend_from_slice(&self.max_bitrate.to_be_bytes());

        // videoCompression: u8
        bytes.push(self.video_compression);

        // histogramEnable, histogramUpdateCycle, usedVideoComponent, dataType, binSize: u8
        bytes.push(self.histogram_enable);
        bytes.push(self.histogram_update_cycle);
        bytes.push(self.used_video_component);
        bytes.push(self.data_type);
        bytes.push(self.bin_size);

        // numberOfBins: 大端序 u16（C++ 中已转）
        bytes.extend_from_slice(&self.number_of_bins.to_be_bytes());

        // blockEnable: bool (u8)
        bytes.push(if self.block_enable { 1 } else { 0 });

        // brightness, contrast, saturation, sharpness: u8
        bytes.push(self.brightness);
        bytes.push(self.contrast);
        bytes.push(self.saturation);
        bytes.push(self.sharpness);

        // streamAtBoot, algStatusIndicator: bool (u8)
        bytes.push(if self.stream_at_boot { 1 } else { 0 });
        bytes.push(if self.alg_status_indicator { 1 } else { 0 });

        bytes
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - 至少 45 字节的字节数组
    ///
    /// # 错误
    ///
    /// 当 `bytes.len() < 45` 时返回 `InsufficientBuffer`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 45 {
            return Err(SomeIPError::insufficient_buffer(45, bytes.len()));
        }

        let index = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let quad1 = u16::from_ne_bytes([bytes[4], bytes[5]]);
        let quad2 = u16::from_ne_bytes([bytes[6], bytes[7]]);
        let quad3 = u16::from_ne_bytes([bytes[8], bytes[9]]);
        let quad4 = u16::from_ne_bytes([bytes[10], bytes[11]]);

        let enable = bytes[12];
        let trans_method = bytes[13];
        let trans_cycle = u16::from_be_bytes([bytes[14], bytes[15]]);
        let width = u16::from_be_bytes([bytes[16], bytes[17]]);
        let height = u16::from_be_bytes([bytes[18], bytes[19]]);
        let frame_rate = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

        let interlaced = bytes[24];
        let color_space = bytes[25];
        let max_bitrate = u32::from_be_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]);

        let video_compression = bytes[30];
        let histogram_enable = bytes[31];
        let histogram_update_cycle = bytes[32];
        let used_video_component = bytes[33];
        let data_type = bytes[34];
        let bin_size = bytes[35];

        let number_of_bins = u16::from_be_bytes([bytes[36], bytes[37]]);

        let block_enable = bytes[38] != 0;
        let brightness = bytes[39];
        let contrast = bytes[40];
        let saturation = bytes[41];
        let sharpness = bytes[42];
        let stream_at_boot = bytes[43] != 0;
        let alg_status_indicator = bytes[44] != 0;

        Ok(SetCameraROIPayload {
            index,
            quad1,
            quad2,
            quad3,
            quad4,
            enable,
            trans_method,
            trans_cycle,
            width,
            height,
            frame_rate,
            interlaced,
            color_space,
            max_bitrate,
            video_compression,
            histogram_enable,
            histogram_update_cycle,
            used_video_component,
            data_type,
            bin_size,
            number_of_bins,
            block_enable,
            brightness,
            contrast,
            saturation,
            sharpness,
            stream_at_boot,
            alg_status_indicator,
        })
    }
}

impl Payload for SetCameraROIPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::SetROI
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for SetCameraROIPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for SetCameraROIPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

/// GetCameraROIPayload（获取摄像头 ROI 区域）。
///
/// 对应 C++ `GetCameraROIPayload`（someip_client.h:227-268）。
///
/// 字段与 `SetCameraROIPayload` 完全相同，仅默认值不同：
/// - `quad3 = 1920`（Set 中为 `0`）
/// - `quad4 = 1080`（Set 中为 `0`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCameraROIPayload {
    /// ROI 区域坐标 1（默认 `0`）
    pub quad1: u16,
    /// ROI 区域坐标 2（默认 `0`）
    pub quad2: u16,
    /// ROI 区域坐标 3（默认 `1920`）
    pub quad3: u16,
    /// ROI 区域坐标 4（默认 `1080`）
    pub quad4: u16,
    /// 使能（默认 `1`）
    pub enable: u8,
    /// 传输方法（默认 `0`）
    pub trans_method: u8,
    /// 传输周期（大端序 u16，默认 `0`）
    pub trans_cycle: u16,
    /// 宽度（大端序 u16，默认 `1280`）
    pub width: u16,
    /// 高度（大端序 u16，默认 `800`）
    pub height: u16,
    /// 帧率（大端序 u32，默认 `30 << 16`）
    pub frame_rate: u32,
    /// 隔行扫描（默认 `0`）
    pub interlaced: u8,
    /// 色彩空间（默认 `0x80`）
    pub color_space: u8,
    /// 最大码率（大端序 u32，默认 `10`）
    pub max_bitrate: u32,
    /// 视频编码（默认 `0x02`）
    pub video_compression: u8,
    /// 直方图使能（默认 `0x00`）
    pub histogram_enable: u8,
    /// 直方图更新周期（默认 `0x00`）
    pub histogram_update_cycle: u8,
    /// 使用的视频组件（默认 `0x00`）
    pub used_video_component: u8,
    /// 数据类型（默认 `0x04`）
    pub data_type: u8,
    /// 箱大小（默认 `0x01`）
    pub bin_size: u8,
    /// 箱数量（大端序 u16，默认 `0x00`）
    pub number_of_bins: u16,
    /// 块使能（默认 `true`）
    pub block_enable: bool,
    /// 亮度（默认 `50`）
    pub brightness: u8,
    /// 对比度（默认 `50`）
    pub contrast: u8,
    /// 饱和度（默认 `50`）
    pub saturation: u8,
    /// 锐度（默认 `50`）
    pub sharpness: u8,
    /// 启动时流式传输（默认 `false`）
    pub stream_at_boot: bool,
    /// 算法状态指示器（默认 `true`）
    pub alg_status_indicator: bool,
}

impl GetCameraROIPayload {
    /// 创建新的 GetCameraROIPayload。
    pub fn new(
        quad1: u16,
        quad2: u16,
        quad3: u16,
        quad4: u16,
        enable: u8,
        trans_method: u8,
        trans_cycle: u16,
        width: u16,
        height: u16,
        frame_rate: u32,
        interlaced: u8,
        color_space: u8,
        max_bitrate: u32,
        video_compression: u8,
        histogram_enable: u8,
        histogram_update_cycle: u8,
        used_video_component: u8,
        data_type: u8,
        bin_size: u8,
        number_of_bins: u16,
        block_enable: bool,
        brightness: u8,
        contrast: u8,
        saturation: u8,
        sharpness: u8,
        stream_at_boot: bool,
        alg_status_indicator: bool,
    ) -> Self {
        GetCameraROIPayload {
            quad1,
            quad2,
            quad3,
            quad4,
            enable,
            trans_method,
            trans_cycle: trans_cycle.to_be(),
            width: width.to_be(),
            height: height.to_be(),
            frame_rate: frame_rate.to_be(),
            interlaced,
            color_space,
            max_bitrate: max_bitrate.to_be(),
            video_compression,
            histogram_enable,
            histogram_update_cycle,
            used_video_component,
            data_type,
            bin_size,
            number_of_bins: number_of_bins.to_be(),
            block_enable,
            brightness,
            contrast,
            saturation,
            sharpness,
            stream_at_boot,
            alg_status_indicator,
        }
    }

    /// 返回默认 GetCameraROIPayload（与 C++ 默认值一致）。
    pub fn default_payload() -> Self {
        GetCameraROIPayload {
            quad1: 0,
            quad2: 0,
            quad3: 1920,
            quad4: 1080,
            enable: 1,
            trans_method: 0,
            trans_cycle: 0_u16.to_be(),
            width: 1280_u16.to_be(),
            height: 800_u16.to_be(),
            frame_rate: (30u32 << 16).to_be(),
            interlaced: 0,
            color_space: 0x80,
            max_bitrate: 10_u32.to_be(),
            video_compression: 0x02,
            histogram_enable: 0x00,
            histogram_update_cycle: 0x00,
            used_video_component: 0x00,
            data_type: 0x04,
            bin_size: 0x01,
            number_of_bins: 0_u16.to_be(),
            block_enable: true,
            brightness: 50,
            contrast: 50,
            saturation: 50,
            sharpness: 50,
            stream_at_boot: false,
            alg_status_indicator: true,
        }
    }

    /// 序列化为字节数组（与 SetCameraROIPayload 格式相同，但不含 index）。
    ///
    /// 注意：C++ 中 `GetCameraROIPayload` 与 `SetCameraROIPayload` 字段相同，
    /// 但 `SetCameraROIPayload` 有 `index` 字段，而 `GetCameraROIPayload` 没有。
    /// 然而 C++ 代码中两者结构体定义相同（都有 index），只是默认值不同。
    ///
    /// 为保持与 C++ 兼容，此处序列化格式与 `SetCameraROIPayload` 相同（含 index）。
    /// 但实际使用时，Get 请求无 payload，此方法用于解析响应。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(45);

        // index: 默认 0x01 大端序（Get 响应中设备返回的值）
        bytes.extend_from_slice(&0x01_u32.to_be_bytes());

        // quad1-4
        bytes.extend_from_slice(&self.quad1.to_ne_bytes());
        bytes.extend_from_slice(&self.quad2.to_ne_bytes());
        bytes.extend_from_slice(&self.quad3.to_ne_bytes());
        bytes.extend_from_slice(&self.quad4.to_ne_bytes());

        // enable, transMethod
        bytes.push(self.enable);
        bytes.push(self.trans_method);

        // transCycle
        bytes.extend_from_slice(&self.trans_cycle.to_be_bytes());

        // width, height
        bytes.extend_from_slice(&self.width.to_be_bytes());
        bytes.extend_from_slice(&self.height.to_be_bytes());

        // frameRate
        bytes.extend_from_slice(&self.frame_rate.to_be_bytes());

        // interlaced, colorSpace
        bytes.push(self.interlaced);
        bytes.push(self.color_space);

        // maxBitrate
        bytes.extend_from_slice(&self.max_bitrate.to_be_bytes());

        // videoCompression
        bytes.push(self.video_compression);

        // histogramEnable, etc.
        bytes.push(self.histogram_enable);
        bytes.push(self.histogram_update_cycle);
        bytes.push(self.used_video_component);
        bytes.push(self.data_type);
        bytes.push(self.bin_size);

        // numberOfBins
        bytes.extend_from_slice(&self.number_of_bins.to_be_bytes());

        // blockEnable
        bytes.push(if self.block_enable { 1 } else { 0 });

        // brightness, contrast, saturation, sharpness
        bytes.push(self.brightness);
        bytes.push(self.contrast);
        bytes.push(self.saturation);
        bytes.push(self.sharpness);

        // streamAtBoot, algStatusIndicator
        bytes.push(if self.stream_at_boot { 1 } else { 0 });
        bytes.push(if self.alg_status_indicator { 1 } else { 0 });

        bytes
    }

    /// 从字节数组反序列化。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        if bytes.len() < 45 {
            return Err(SomeIPError::insufficient_buffer(45, bytes.len()));
        }

        // 跳过 index (4 bytes)
        let quad1 = u16::from_ne_bytes([bytes[4], bytes[5]]);
        let quad2 = u16::from_ne_bytes([bytes[6], bytes[7]]);
        let quad3 = u16::from_ne_bytes([bytes[8], bytes[9]]);
        let quad4 = u16::from_ne_bytes([bytes[10], bytes[11]]);

        let enable = bytes[12];
        let trans_method = bytes[13];
        let trans_cycle = u16::from_be_bytes([bytes[14], bytes[15]]);
        let width = u16::from_be_bytes([bytes[16], bytes[17]]);
        let height = u16::from_be_bytes([bytes[18], bytes[19]]);
        let frame_rate = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

        let interlaced = bytes[24];
        let color_space = bytes[25];
        let max_bitrate = u32::from_be_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]);

        let video_compression = bytes[30];
        let histogram_enable = bytes[31];
        let histogram_update_cycle = bytes[32];
        let used_video_component = bytes[33];
        let data_type = bytes[34];
        let bin_size = bytes[35];

        let number_of_bins = u16::from_be_bytes([bytes[36], bytes[37]]);

        let block_enable = bytes[38] != 0;
        let brightness = bytes[39];
        let contrast = bytes[40];
        let saturation = bytes[41];
        let sharpness = bytes[42];
        let stream_at_boot = bytes[43] != 0;
        let alg_status_indicator = bytes[44] != 0;

        Ok(GetCameraROIPayload {
            quad1,
            quad2,
            quad3,
            quad4,
            enable,
            trans_method,
            trans_cycle,
            width,
            height,
            frame_rate,
            interlaced,
            color_space,
            max_bitrate,
            video_compression,
            histogram_enable,
            histogram_update_cycle,
            used_video_component,
            data_type,
            bin_size,
            number_of_bins,
            block_enable,
            brightness,
            contrast,
            saturation,
            sharpness,
            stream_at_boot,
            alg_status_indicator,
        })
    }
}

impl Payload for GetCameraROIPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::GetROI
    }

    fn encode(&self) -> Vec<u8> {
        // Get 请求无 payload，但响应有 payload
        // 此处返回序列化结果，用于解析响应
        self.to_bytes()
    }
}

impl PayloadCodec for GetCameraROIPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for GetCameraROIPayload {
    fn default() -> Self {
        Self::default_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_camera_roi_payload_to_bytes_roundtrip() {
        let payload = SetCameraROIPayload::default_payload();
        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 45);

        let parsed = SetCameraROIPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.width, payload.width);
    }

    #[test]
    fn test_get_camera_roi_payload_default() {
        let payload = GetCameraROIPayload::default_payload();
        assert_eq!(payload.quad3, 1920);
        assert_eq!(payload.quad4, 1080);
    }
}
