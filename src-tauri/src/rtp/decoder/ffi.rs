//! FFI 接口定义
//!
//! 对接 C++/FFmpeg 的 FFI 接口
//!
//! # C++ 端接口设计
//!
//! C++ 端需要提供以下 C 风格接口：
//!
//! ```cpp
//! // 创建解码器
//! DecoderHandle decoder_create(int codec_type, int pixel_format);
//!
//! // 销毁解码器
//! void decoder_destroy(DecoderHandle handle);
//!
//! // 解码一帧
//! int decoder_decode(DecoderHandle handle, const uint8_t* data, size_t size,
//!                    int64_t pts, int64_t dts, int keyframe);
//!
//! // 获取解码后的帧
//! int decoder_get_frame(DecoderHandle handle, DecodedFrame* frame);
//!
//! // 刷新解码器
//! int decoder_flush(DecoderHandle handle);
//!
//! // 重置解码器
//! void decoder_reset(DecoderHandle handle);
//! ```
//!
//! # Feature Flags
//! - `decoder-ffi`: 启用 FFI 实现

use std::ffi::c_void;

/// 解码器句柄（C++ 端的解码器实例指针）
pub type DecoderHandle = *mut c_void;

/// 解码后帧的 C 结构（与 C++ 端对齐）
#[repr(C)]
pub struct DecodedFrame {
    /// 媒体类型（0=Video, 1=Audio）
    pub media_type: i32,
    /// 编解码器类型
    pub codec_type: i32,

    // ========== 视频字段 ==========
    /// 像素格式
    pub pixel_format: i32,
    /// 宽度
    pub width: i32,
    /// 高度
    pub height: i32,
    ///  stride
    pub stride: [i32; 8],
    /// 平面偏移
    pub plane_offset: [i32; 8],
    /// 平面数量
    pub plane_count: i32,

    // ========== 音频字段 ==========
    /// 采样格式
    pub sample_format: i32,
    /// 采样率
    pub sample_rate: i32,
    /// 声道数
    pub channels: i32,
    /// 声道布局
    pub channel_layout: u64,
    /// 每声道采样点数
    pub nb_samples: i32,
    /// 每采样字节数
    pub bytes_per_sample: i32,
    /// 是否为 planar 格式
    pub planar: bool,

    // ========== 数据字段 ==========
    /// 数据指针
    pub data: *const u8,
    /// 数据大小
    pub size: usize,
    /// 时间戳
    pub pts: i64,
    pub dts: i64,
    /// 帧持续时间
    pub duration: i64,
    /// 是否为关键帧
    pub keyframe: bool,
}

/// FFI 错误码
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiErrorCode {
    /// 成功
    Success = 0,
    /// 失败
    Failed = -1,
    /// 无效参数
    InvalidParameter = -2,
    /// 解码错误
    DecodeError = -3,
    /// 缓冲区满
    BufferFull = -4,
    /// 超时
    Timeout = -5,
}

/// FFI 解码器包装器
///
/// 用于包装 C++ 端的解码器实例
pub struct FfiDecoder {
    /// 解码器句柄
    handle: DecoderHandle,
    /// 编解码器类型
    codec_type: i32,
    /// 输出像素格式（视频）
    pixel_format: i32,
    /// 输出采样格式（音频）
    sample_format: i32,
    /// 媒体类型
    media_type: i32,
}

// SAFETY: FfiDecoder 的 handle 是 C++ 端解码器实例的指针。
// 假设 C++ 端实现是线程安全的（或者解码器在同一时间只被一个线程使用），
// 则 FfiDecoder 可以安全地跨线程传递。
// 注意：FfiDecoder 不同步，不应在多线程中同时使用同一个实例。
unsafe impl Send for FfiDecoder {}

impl FfiDecoder {
    /// 创建新的视频 FFI 解码器
    pub fn new_video(codec_type: i32, pixel_format: i32) -> Result<Self, String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_create 函数
            // let handle = unsafe { ffi::decoder_create(codec_type, pixel_format, 0) };
            // if handle.is_null() {
            //     return Err("failed to create video decoder".to_string());
            // }
            // Ok(Self { handle, codec_type, pixel_format, sample_format: 0, media_type: 0 })
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 创建新的音频 FFI 解码器
    pub fn new_audio(codec_type: i32, sample_format: i32) -> Result<Self, String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_create 函数
            // let handle = unsafe { ffi::decoder_create(codec_type, sample_format, 1) };
            // if handle.is_null() {
            //     return Err("failed to create audio decoder".to_string());
            // }
            // Ok(Self { handle, codec_type, pixel_format: 0, sample_format, media_type: 1 })
            return Err("FFI audio decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 解码数据
    pub fn decode(
        &mut self,
        data: &[u8],
        pts: i64,
        dts: i64,
        keyframe: bool,
    ) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_decode 函数
            // let ret = unsafe {
            //     ffi::decoder_decode(
            //         self.handle,
            //         data.as_ptr(),
            //         data.len(),
            //         pts,
            //         dts,
            //         keyframe,
            //     )
            // };
            // if ret != 0 {
            //     return Err(format!("decode failed: {}", ret));
            // }
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 获取解码后的帧
    pub fn get_frame(&mut self) -> Result<Option<DecodedFrame>, String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_get_frame 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 刷新解码器
    pub fn flush(&mut self) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_flush 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 重置解码器
    pub fn reset(&mut self) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_reset 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }
}

impl Drop for FfiDecoder {
    fn drop(&mut self) {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_destroy 函数
            // if !self.handle.is_null() {
            //     unsafe { ffi::decoder_destroy(self.handle) };
            // }
        }
    }
}

impl DecodedFrame {
    /// 将 DecodedFrame 转换为 MediaFrame
    pub fn to_media_frame(&self, data: bytes::Bytes) -> crate::rtp::decoder::frame::MediaFrame {
        use crate::rtp::decoder::types::{MediaType, PixelFormat, SampleFormat};
        use crate::rtp::decoder::frame::MediaFrame;

        let media_type = MediaType::from_u32(self.media_type as u32);

        if media_type == MediaType::Video {
            // 视频帧
            MediaFrame {
                media_type,
                pixel_format: PixelFormat::from_u32(self.pixel_format as u32),
                width: self.width,
                height: self.height,
                stride: self.stride,
                plane_offset: self.plane_offset,
                plane_count: self.plane_count,
                sample_format: SampleFormat::Unknown,
                sample_rate: 0,
                channels: 0,
                channel_layout: 0,
                nb_samples: 0,
                bytes_per_sample: 0,
                planar: false,
                pts: self.pts,
                dts: self.dts,
                duration: self.duration,
                keyframe: self.keyframe,
                data,
                backend: None,
            }
        } else {
            // 音频帧
            MediaFrame {
                media_type,
                pixel_format: PixelFormat::Unknown,
                width: 0,
                height: 0,
                stride: [0; 8],
                plane_offset: [0; 8],
                plane_count: 0,
                sample_format: SampleFormat::from_u32(self.sample_format as u32),
                sample_rate: self.sample_rate,
                channels: self.channels,
                channel_layout: self.channel_layout,
                nb_samples: self.nb_samples,
                bytes_per_sample: self.bytes_per_sample,
                planar: self.planar,
                pts: self.pts,
                dts: self.dts,
                duration: self.duration,
                keyframe: self.keyframe,
                data,
                backend: None,
            }
        }
    }
}

/// 声明外部 C 函数（C++ 端实现）
#[cfg(feature = "decoder-ffi")]
#[allow(non_snake_case)]
mod ffi {
    use super::*;

    extern "C" {
        /// 创建解码器
        pub fn decoder_create(codec_type: i32, pixel_format: i32) -> DecoderHandle;

        /// 销毁解码器
        pub fn decoder_destroy(handle: DecoderHandle);

        /// 解码一帧
        pub fn decoder_decode(
            handle: DecoderHandle,
            data: *const u8,
            size: usize,
            pts: i64,
            dts: i64,
            keyframe: bool,
        ) -> i32;

        /// 获取解码后的帧
        pub fn decoder_get_frame(handle: DecoderHandle, frame: *mut DecodedFrame) -> i32;

        /// 刷新解码器
        pub fn decoder_flush(handle: DecoderHandle) -> i32;

        /// 重置解码器
        pub fn decoder_reset(handle: DecoderHandle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_decoder_new() {
        // 默认应该返回错误（未实现或 feature 未启用）
        let result = FfiDecoder::new_video(0, 1);
        assert!(result.is_err());
    }
}
