//! 媒体数据包和帧定义
//!
//! 对齐 C++ 端的 MediaPacket 和 MediaFrame 类

use bytes::Bytes;
use crate::rtp::decoder::types::{MediaType, CodecType, PixelFormat, BackendHandle};

/// 编码数据包（输入到解码器）
///
/// 对齐 C++ 端 MediaPacket 类
#[derive(Debug, Clone)]
pub struct MediaPacket {
    /// 媒体流类型
    pub media_type: MediaType,
    /// 编码格式
    pub codec_type: CodecType,
    /// 显示时间戳（微秒）
    pub pts: i64,
    /// 解码时间戳（微秒）
    pub dts: i64,
    /// 是否为关键帧
    pub keyframe: bool,
    /// 编码数据载荷
    pub data: Bytes,
    /// 后端引擎句柄
    pub backend: Option<BackendHandle>,
}

impl MediaPacket {
    /// 创建新的 MediaPacket
    pub fn new(codec_type: CodecType, data: Bytes) -> Self {
        Self {
            media_type: if codec_type.is_video() {
                MediaType::Video
            } else if codec_type.is_audio() {
                MediaType::Audio
            } else {
                MediaType::Unknown
            },
            codec_type,
            pts: 0,
            dts: 0,
            keyframe: false,
            data,
            backend: None,
        }
    }

    /// 设置时间戳
    pub fn with_timestamps(mut self, pts: i64, dts: i64) -> Self {
        self.pts = pts;
        self.dts = dts;
        self
    }

    /// 标记为关键帧
    pub fn with_keyframe(mut self, keyframe: bool) -> Self {
        self.keyframe = keyframe;
        self
    }

    /// 设置后端句柄
    pub fn with_backend(mut self, backend: BackendHandle) -> Self {
        self.backend = Some(backend);
        self
    }

    /// 获取数据切片
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// 获取数据长度
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// 解码后帧（解码器输出）
///
/// 对齐 C++ 端 MediaFrame 类
#[derive(Debug, Clone)]
pub struct MediaFrame {
    /// 媒体类型
    pub media_type: MediaType,
    /// 像素格式
    pub pixel_format: PixelFormat,
    /// 图像宽度（像素）
    pub width: i32,
    /// 图像高度（像素）
    pub height: i32,

    /// 平面行跨度（字节）
    /// stride[0]: Y/R 平面, stride[1]: U/G 平面, stride[2]: V/B 平面, stride[3]: Alpha 平面
    pub stride: [i32; 4],

    /// 平面数据偏移（字节）
    /// plane_offset[0]: Y/R 平面, plane_offset[1]: U/G 平面, plane_offset[2]: V/B 平面, plane_offset[3]: Alpha 平面
    pub plane_offset: [i32; 4],

    /// 平面数量
    /// 1: packed (RGB/RGBA/GRAY)
    /// 2: semi-planar (NV12/NV21)
    /// 3: planar (YUV420P/YUV422P/YUV444P)
    pub plane_count: i32,

    /// 显示时间戳（微秒）
    pub pts: i64,
    /// 解码时间戳（微秒）
    pub dts: i64,
    /// 帧持续时间（微秒）
    pub duration: i64,

    /// 是否为关键帧
    pub keyframe: bool,

    /// packed 图像数据
    pub data: Bytes,

    /// 后端引擎句柄
    pub backend: Option<BackendHandle>,
}

impl MediaFrame {
    /// 创建新的 MediaFrame
    pub fn new(pixel_format: PixelFormat, width: i32, height: i32, data: Bytes) -> Self {
        // 自动计算 stride 和 plane_offset
        let (stride, plane_offset, plane_count) = Self::calc_stride_and_offset(pixel_format, width, height);

        Self {
            media_type: MediaType::Video,
            pixel_format,
            width,
            height,
            stride,
            plane_offset,
            plane_count,
            pts: 0,
            dts: 0,
            duration: 0,
            keyframe: false,
            data,
            backend: None,
        }
    }

    /// 设置时间戳
    pub fn with_timestamps(mut self, pts: i64, dts: i64) -> Self {
        self.pts = pts;
        self.dts = dts;
        self
    }

    /// 设置帧持续时间
    pub fn with_duration(mut self, duration: i64) -> Self {
        self.duration = duration;
        self
    }

    /// 标记为关键帧
    pub fn with_keyframe(mut self, keyframe: bool) -> Self {
        self.keyframe = keyframe;
        self
    }

    /// 设置后端句柄
    pub fn with_backend(mut self, backend: BackendHandle) -> Self {
        self.backend = Some(backend);
        self
    }

    /// 计算 stride 和 plane_offset
    fn calc_stride_and_offset(
        pixel_format: PixelFormat,
        width: i32,
        height: i32,
    ) -> ([i32; 4], [i32; 4], i32) {
        let mut stride = [0; 4];
        let mut plane_offset = [0; 4];
        let mut plane_count = 0;

        match pixel_format {
            PixelFormat::RGBA | PixelFormat::BGRA => {
                // packed 格式
                stride[0] = width * 4;
                plane_offset[0] = 0;
                plane_count = 1;
            }
            PixelFormat::RGB | PixelFormat::BGR => {
                // packed 格式
                stride[0] = width * 3;
                plane_offset[0] = 0;
                plane_count = 1;
            }
            PixelFormat::YUV420P => {
                // planar 格式: Y, U, V
                let y_size = width * height;
                let uv_width = (width + 1) / 2;
                let uv_height = (height + 1) / 2;
                stride[0] = width;
                stride[1] = uv_width;
                stride[2] = uv_width;
                plane_offset[0] = 0;
                plane_offset[1] = y_size;
                plane_offset[2] = y_size + uv_width * uv_height;
                plane_count = 3;
            }
            PixelFormat::NV12 | PixelFormat::NV21 => {
                // semi-planar 格式: Y, UV
                let y_size = width * height;
                stride[0] = width;
                stride[1] = width; // UV 交错，stride 与 Y 相同
                plane_offset[0] = 0;
                plane_offset[1] = y_size;
                plane_count = 2;
            }
            PixelFormat::YUV422P => {
                // planar 格式: Y, U, V
                let y_size = width * height;
                let uv_width = (width + 1) / 2;
                stride[0] = width;
                stride[1] = uv_width;
                stride[2] = uv_width;
                plane_offset[0] = 0;
                plane_offset[1] = y_size;
                plane_offset[2] = y_size + uv_width * height;
                plane_count = 3;
            }
            PixelFormat::YUY2 | PixelFormat::UYVY => {
                // packed 格式
                stride[0] = width * 2;
                plane_offset[0] = 0;
                plane_count = 1;
            }
            PixelFormat::YUV444P => {
                // planar 格式: Y, U, V
                let y_size = width * height;
                stride[0] = width;
                stride[1] = width;
                stride[2] = width;
                plane_offset[0] = 0;
                plane_offset[1] = y_size;
                plane_offset[2] = y_size * 2;
                plane_count = 3;
            }
            PixelFormat::GRAY8 => {
                // 单平面
                stride[0] = width;
                plane_offset[0] = 0;
                plane_count = 1;
            }
            PixelFormat::GRAY16 => {
                // 单平面
                stride[0] = width * 2;
                plane_offset[0] = 0;
                plane_count = 1;
            }
            _ => {
                // 未知格式
                stride[0] = 0;
                plane_offset[0] = 0;
                plane_count = 0;
            }
        }

        (stride, plane_offset, plane_count)
    }

    /// 获取数据切片
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// 获取数据长度
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 获取指定平面的数据切片
    pub fn plane_data(&self, plane_index: usize) -> Option<&[u8]> {
        if plane_index >= self.plane_count as usize {
            return None;
        }

        let offset = self.plane_offset[plane_index] as usize;
        let next_offset = if plane_index + 1 < self.plane_count as usize {
            self.plane_offset[plane_index + 1] as usize
        } else {
            self.data.len()
        };

        if offset >= self.data.len() || next_offset > self.data.len() {
            return None;
        }

        Some(&self.data[offset..next_offset])
    }

    /// 获取帧大小（字节）
    pub fn frame_size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_media_packet() {
        let data = Bytes::from_static(b"test data");
        let packet = MediaPacket::new(CodecType::H264, data.clone())
            .with_timestamps(1000, 1000)
            .with_keyframe(true);

        assert_eq!(packet.media_type, MediaType::Video);
        assert_eq!(packet.codec_type, CodecType::H264);
        assert_eq!(packet.pts, 1000);
        assert_eq!(packet.keyframe, true);
        assert_eq!(packet.data, data);
    }

    #[test]
    fn test_media_frame_rgb() {
        let width = 1920;
        let height = 1080;
        let data = Bytes::from(vec![0u8; (width * height * 3) as usize]);
        let frame = MediaFrame::new(PixelFormat::RGB, width, height, data);

        assert_eq!(frame.pixel_format, PixelFormat::RGB);
        assert_eq!(frame.width, width);
        assert_eq!(frame.height, height);
        assert_eq!(frame.plane_count, 1);
        assert_eq!(frame.stride[0], width * 3);
    }

    #[test]
    fn test_media_frame_yuv420p() {
        let width = 1920;
        let height = 1080;
        let data = Bytes::from(vec![0u8; (width * height * 3 / 2) as usize]);
        let frame = MediaFrame::new(PixelFormat::YUV420P, width, height, data);

        assert_eq!(frame.pixel_format, PixelFormat::YUV420P);
        assert_eq!(frame.plane_count, 3);
        assert_eq!(frame.stride[0], width);
        assert_eq!(frame.stride[1], (width + 1) / 2);

        // 检查平面数据
        assert!(frame.plane_data(0).is_some());
        assert!(frame.plane_data(1).is_some());
        assert!(frame.plane_data(2).is_some());
        assert!(frame.plane_data(3).is_none());
    }

    #[test]
    fn test_media_frame_nv12() {
        let width = 1920;
        let height = 1080;
        let data = Bytes::from(vec![0u8; (width * height * 3 / 2) as usize]);
        let frame = MediaFrame::new(PixelFormat::NV12, width, height, data);

        assert_eq!(frame.pixel_format, PixelFormat::NV12);
        assert_eq!(frame.plane_count, 2);
        assert_eq!(frame.stride[0], width);
        assert_eq!(frame.stride[1], width);

        // 检查平面数据
        assert!(frame.plane_data(0).is_some());
        assert!(frame.plane_data(1).is_some());
        assert!(frame.plane_data(2).is_none());
    }
}
